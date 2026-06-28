//! A validated internally-tagged `oneOf` schema: a non-empty name plus a
//! non-empty discriminator tag and a non-empty list of variants whose
//! `wire_name` values are all distinct.
//!
//! Each [`OneOfVariant`] holds the `SchemaName` of an inner struct schema
//! that is registered separately in [`crate::Schemas`]. The cross-schema
//! invariant — that the referenced inner schema is itself a
//! [`crate::Schema::Object`] (not another enum-shaped schema) — is enforced
//! at [`crate::Schemas`]-building time, since at construction time we do
//! not yet know which sibling schemas have been registered.

use crate::description::normalize_description;
use crate::error::Error;
use crate::schema_name::SchemaName;

/// One arm of an internally-tagged [`OneOfSchema`].
///
/// - `wire_name` is the tag value that distinguishes this variant on the
///   wire. In OAS output it appears as the `const`-style `enum: [<wire_name>]`
///   constraint on the synthesized tag property inside each `allOf` arm.
///   The constructor of [`OneOfSchema`] rejects empty `wire_name` values
///   and duplicates across the variants of one schema.
/// - `inner` is the schema name of the newtype's inner struct, used as the
///   target of the `$ref` inside the same `allOf` arm. The build-time
///   transitive-closure check verifies that the referenced schema is
///   registered **and** is a struct schema (`Schema::Object`).
/// - `description` is the optional per-variant doc comment. OAS has no
///   per-variant description slot in `oneOf`, so the rendering side
///   composes this into the enclosing schema's `description` as a bullet
///   row, mirroring the unit-variant enum's behaviour.
///
/// The fields are `pub` because the type's contract is its shape, not
/// behavior. Mutating after construction is the caller's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOfVariant {
    pub wire_name: String,
    pub inner: SchemaName,
    pub description: Option<String>,
}

impl OneOfVariant {
    /// Builds a variant with no description; the description can be added
    /// later through [`OneOfVariant::with_description`]. The inner
    /// reference is taken as already-validated by [`SchemaName::new`] at
    /// the call site.
    pub fn new(wire_name: impl Into<String>, inner: SchemaName) -> Self {
        Self {
            wire_name: wire_name.into(),
            inner,
            description: None,
        }
    }

    /// Attaches a per-variant description, normalizing empty or
    /// whitespace-only input to `None`.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description.and_then(normalize_description);
        self
    }
}

/// An internally-tagged `oneOf` schema in its validated form.
///
/// Validation happens once, in [`OneOfSchema::new`]:
///
/// - the schema name is non-empty and OAS-component-legal,
/// - `tag` (the discriminator property name) is non-empty,
/// - `variants` is non-empty,
/// - every variant's `wire_name` is non-empty,
/// - the `wire_name` values are pairwise distinct.
///
/// The "inner reference points at a struct schema" invariant is
/// **cross-schema** and therefore checked at
/// [`crate::Schemas`]-build time, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOfSchema {
    pub name: SchemaName,
    pub tag: String,
    pub variants: Vec<OneOfVariant>,
    /// Free-form description text. Per-variant descriptions are composed
    /// into this string upstream in `frieze-macros`, mirroring the
    /// unit-variant enum's behaviour, so the rendering side does not need
    /// to know about variant docs separately.
    pub description: Option<String>,
}

impl OneOfSchema {
    /// Builds an internally-tagged `oneOf` schema. The description is
    /// initialised to `None`; use [`OneOfSchema::with_description`] to
    /// attach one.
    pub fn new(
        name: impl Into<String>,
        tag: impl Into<String>,
        variants: Vec<OneOfVariant>,
    ) -> Result<Self, Error> {
        let name = SchemaName::new(name)?;
        let tag = tag.into();
        if tag.is_empty() {
            return Err(Error::EmptyOneOfTag(name.into_string()));
        }
        if variants.is_empty() {
            return Err(Error::NoVariants(name.into_string()));
        }
        let mut seen: Vec<String> = Vec::with_capacity(variants.len());
        for variant in &variants {
            if variant.wire_name.is_empty() {
                return Err(Error::EmptyVariantValue(name.into_string()));
            }
            if seen.iter().any(|existing| existing == &variant.wire_name) {
                return Err(Error::DuplicateVariantValue {
                    schema: name.into_string(),
                    value: variant.wire_name.clone(),
                });
            }
            seen.push(variant.wire_name.clone());
        }
        Ok(Self {
            name,
            tag,
            variants,
            description: None,
        })
    }

    /// Attaches a top-level description to the schema, normalizing empty
    /// or whitespace-only input to `None`.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description.and_then(normalize_description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(name: &str, inner: &str) -> OneOfVariant {
        OneOfVariant::new(name, SchemaName::new(inner).unwrap())
    }

    #[test]
    fn rejects_empty_name() {
        let err = OneOfSchema::new("", "kind", vec![variant("Login", "LoginData")]).unwrap_err();
        assert_eq!(err, Error::EmptySchemaName);
    }

    #[test]
    fn rejects_empty_tag() {
        let err = OneOfSchema::new("Event", "", vec![variant("Login", "LoginData")]).unwrap_err();
        assert_eq!(err, Error::EmptyOneOfTag("Event".into()));
    }

    #[test]
    fn rejects_no_variants() {
        let err = OneOfSchema::new("Event", "kind", Vec::<OneOfVariant>::new()).unwrap_err();
        assert_eq!(err, Error::NoVariants("Event".into()));
    }

    #[test]
    fn rejects_empty_wire_name() {
        let err = OneOfSchema::new("Event", "kind", vec![variant("", "LoginData")]).unwrap_err();
        assert_eq!(err, Error::EmptyVariantValue("Event".into()));
    }

    #[test]
    fn rejects_duplicate_wire_names() {
        let err = OneOfSchema::new(
            "Event",
            "kind",
            vec![variant("Login", "LoginData"), variant("Login", "OtherData")],
        )
        .unwrap_err();
        assert_eq!(
            err,
            Error::DuplicateVariantValue {
                schema: "Event".into(),
                value: "Login".into(),
            }
        );
    }

    #[test]
    fn preserves_declaration_order() {
        let schema = OneOfSchema::new(
            "Event",
            "kind",
            vec![
                variant("Login", "LoginData"),
                variant("Logout", "LogoutData"),
            ],
        )
        .unwrap();
        let names: Vec<&str> = schema
            .variants
            .iter()
            .map(|v| v.wire_name.as_str())
            .collect();
        assert_eq!(names, vec!["Login", "Logout"]);
    }

    #[test]
    fn description_is_none_by_default() {
        let schema =
            OneOfSchema::new("Event", "kind", vec![variant("Login", "LoginData")]).unwrap();
        assert_eq!(schema.description, None);
    }

    #[test]
    fn with_description_attaches_text() {
        let schema = OneOfSchema::new("Event", "kind", vec![variant("Login", "LoginData")])
            .unwrap()
            .with_description(Some("an event".into()));
        assert_eq!(schema.description.as_deref(), Some("an event"));
    }

    #[test]
    fn with_description_normalizes_blank_to_none() {
        let schema = OneOfSchema::new("Event", "kind", vec![variant("Login", "LoginData")])
            .unwrap()
            .with_description(Some("  \n ".into()));
        assert_eq!(schema.description, None);
    }

    #[test]
    fn variant_with_description_normalizes_blank() {
        let v = OneOfVariant::new("Login", SchemaName::new("LoginData").unwrap())
            .with_description(Some("   ".into()));
        assert_eq!(v.description, None);
    }
}
