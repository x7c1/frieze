//! A validated property attached to a schema.

use serde::{Deserialize, Serialize};

use crate::description::normalize_description;
use crate::error::Error;
use crate::presence::Presence;
use crate::property_name::PropertyName;
use crate::property_type::PropertyType;

/// A property attached to a schema, in its validated form.
///
/// A property is the quadruple (name, type, presence, description).
/// **Presence** controls whether the field name is listed under the
/// schema's `required` array; **nullability** of the value is encoded
/// inside [`PropertyType`] via [`PropertyType::Nullable`]. The two axes
/// are independent — see [`Presence`] for the four combinations they
/// enumerate. **Description** carries optional free-form text sourced
/// from the originating Rust `///` doc-comment; it is rendered as the
/// `description` field of the per-property OAS schema when present.
///
/// Validation happens once, in [`Property::new`]. The fields are `pub`
/// because the type's contract is its shape, not behavior: callers may read
/// or (re-)assign fields directly. Maintaining the documented invariants on
/// a value built via struct-literal or post-construction mutation is the
/// caller's responsibility — the constructor is the only place that checks
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub name: PropertyName,
    /// `ty` follows the Rust AST convention (e.g. `syn::Field::ty`, the `ty`
    /// fragment specifier in `macro_rules!`). `frieze-macros` reads
    /// `syn::Field::ty` to populate this field, so the naming is consistent
    /// across the AST boundary. Kept short and idiomatic rather than
    /// `type_` / `tpe` / `r#type`.
    pub ty: PropertyType,
    /// Whether the property key is required to appear in the serialized
    /// object. Drives the schema's `required` array; the value-level
    /// nullability is independent and lives inside [`PropertyType`].
    pub presence: Presence,
    /// Free-form description text sourced from the originating Rust
    /// `///` doc-comment. Empty / whitespace-only inputs are normalized
    /// to `None` at the [`Property::with_description`] entry point so
    /// the renderer never emits an empty `description` key (see the
    /// empty-container omission rule).
    pub description: Option<String>,
}

impl Property {
    /// Builds a property, rejecting empty names. The description is
    /// initialized to `None`; use [`Property::with_description`] to
    /// attach one.
    pub fn new(
        name: impl Into<String>,
        ty: PropertyType,
        presence: Presence,
    ) -> Result<Self, Error> {
        Ok(Self {
            name: PropertyName::new(name)?,
            ty,
            presence,
            description: None,
        })
    }

    /// Attaches a description to the property, normalizing empty or
    /// whitespace-only input to `None` so the renderer never emits an
    /// empty `description` key.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description.and_then(normalize_description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        let err = Property::new("", PropertyType::Int64, Presence::Required).unwrap_err();
        assert_eq!(err, Error::EmptyPropertyName);
    }

    #[test]
    fn accepts_required_property() {
        let property = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        assert_eq!(property.name.as_str(), "id");
        assert_eq!(property.ty, PropertyType::Int64);
        assert_eq!(property.presence, Presence::Required);
        assert_eq!(property.description, None);
    }

    #[test]
    fn accepts_optional_property() {
        let property = Property::new(
            "nickname",
            PropertyType::Nullable(Box::new(PropertyType::String)),
            Presence::Optional,
        )
        .unwrap();
        assert_eq!(property.name.as_str(), "nickname");
        assert_eq!(
            property.ty,
            PropertyType::Nullable(Box::new(PropertyType::String))
        );
        assert_eq!(property.presence, Presence::Optional);
    }

    #[test]
    fn with_description_attaches_text() {
        let property = Property::new("name", PropertyType::String, Presence::Required)
            .unwrap()
            .with_description(Some("display name".into()));
        assert_eq!(property.description.as_deref(), Some("display name"));
    }

    #[test]
    fn with_description_normalizes_empty_to_none() {
        let property = Property::new("name", PropertyType::String, Presence::Required)
            .unwrap()
            .with_description(Some(String::new()));
        assert_eq!(property.description, None);
    }

    #[test]
    fn with_description_normalizes_whitespace_only_to_none() {
        let property = Property::new("name", PropertyType::String, Presence::Required)
            .unwrap()
            .with_description(Some("   \n\t  ".into()));
        assert_eq!(property.description, None);
    }

    #[test]
    fn with_description_passes_through_none() {
        let property = Property::new("name", PropertyType::String, Presence::Required)
            .unwrap()
            .with_description(None);
        assert_eq!(property.description, None);
    }
}
