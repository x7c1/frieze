//! Builder that collects [`Schema`] implementations into a validated
//! [`frieze_model::Schemas`].

use frieze_model::{
    primitive_property_type_for, Error, PropertyType, Schema as ModelSchema, SchemaName, Schemas,
};

use crate::schema::{IsRegistrable, Schema};

/// In-progress collection of schemas.
#[derive(Debug, Default)]
pub struct SchemasBuilder {
    schemas: Vec<ModelSchema>,
}

impl SchemasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()` and recursively
    /// registers every type that `T`'s schema references.
    ///
    /// `T` must implement [`IsRegistrable`] — this rejects primitive
    /// scalars at compile time (`Schemas::add::<i64>()` fails to
    /// compile), since primitive scalars implement [`Schema`] only so
    /// they can appear as generic arguments and are not standalone OAS
    /// schema entries. `#[derive(Schema)]` emits the `IsRegistrable`
    /// impl for struct and enum inputs.
    ///
    /// The traversal is performed by [`Schema::register_into`]: the
    /// derived impl walks each field type's `register_into`, so a single
    /// `add::<Foo>()` call pulls in `Foo` together with every nested
    /// struct / enum / generic instance reachable from `Foo`'s fields.
    /// Calls are idempotent — adding the same root twice, or having a
    /// root reachable through multiple paths, leaves only one entry per
    /// name in the resulting `Schemas`.
    pub fn add<T: IsRegistrable>(mut self) -> Self {
        <T as Schema>::register_into(&mut self);
        self
    }

    /// Pushes `schema` into the in-progress collection only if no schema
    /// with the same registration name is already present.
    ///
    /// `Schema::Scalar` entries (and anything else whose
    /// [`ModelSchema::name`] returns `None`) are always appended — they
    /// have no key to dedup on and are filtered out at
    /// [`Schemas::new`] anyway.
    ///
    /// This is the idempotent push primitive used by
    /// [`Schema::register_into`] (both the default impl and the
    /// derive-emitted override) so the same root reached through
    /// multiple paths or via a self-referential cycle (`struct Tree
    /// { children: Vec<Box<Tree>> }`) collapses to a single entry.
    pub fn push_unique(&mut self, schema: ModelSchema) {
        if let Some(name) = schema.name() {
            if self.contains_name(name.as_str()) {
                return;
            }
        }
        self.schemas.push(schema);
    }

    /// Returns `true` if a previously-pushed schema has the same
    /// registration name as `name`.
    ///
    /// The derive-emitted [`Schema::register_into`] uses this as the
    /// early-return guard at the top of the body: `if
    /// builder.contains_name(&Self::name()) { return; }` short-circuits
    /// recursion through self-referential types and multi-path arrival
    /// of the same root.
    pub fn contains_name(&self, name: &str) -> bool {
        self.schemas
            .iter()
            .any(|s| s.name().map(|n| n.as_str() == name).unwrap_or(false))
    }

    /// Finalizes the collection, checking that every `$ref` resolves
    /// to a registered schema.
    ///
    /// References are gathered by walking each property's type tree
    /// (recursing into `Array(...)` and `Nullable(...)`) and each
    /// `oneOf` variant's inner reference. The first ref that points at
    /// a schema not in the collection produces
    /// [`Error::UnresolvedReference`], in declaration order.
    ///
    /// Duplicate-name detection still runs at the domain layer via
    /// [`Schemas::new`], but the builder pushes every entry through
    /// [`Self::push_unique`], so [`Error::DuplicateSchema`] is no
    /// longer reachable through the standard `add` path; it remains
    /// the defensive guarantee at the model layer when a `Schemas`
    /// value is built outside the use-case-layer builder.
    ///
    /// In addition, each `oneOf` variant's inner reference must point at
    /// a struct schema (`Schema::Object`); pointing at another enum
    /// schema (`Schema::StringEnum` or `Schema::OneOf`) is rejected
    /// with [`Error::OneOfVariantInnerNotStruct`] because the
    /// synthesized tag field must merge into an object body, not into a
    /// scalar-shaped or already-discriminated value.
    pub fn build(self) -> Result<Schemas, Error> {
        let schemas = Schemas::new(self.schemas)?;
        for schema in schemas.by_name.values() {
            if let Some(missing) = first_unresolved_in_schema(schema, &schemas) {
                return Err(Error::UnresolvedReference(missing.clone()));
            }
            check_one_of_variants_target_struct_schemas(schema, &schemas)?;
        }
        Ok(schemas)
    }
}

/// Confirms each `oneOf` variant's inner reference resolves to a
/// [`ModelSchema::Object`]. Pointing a oneOf arm at a string-enum or
/// another oneOf would break the internal-tagged shape — the synthesized
/// tag field has nothing to merge into.
fn check_one_of_variants_target_struct_schemas(
    schema: &ModelSchema,
    schemas: &Schemas,
) -> Result<(), Error> {
    let one_of = match schema {
        ModelSchema::OneOf(o) => o,
        ModelSchema::Object(_) | ModelSchema::StringEnum(_) | ModelSchema::Scalar(_) => {
            return Ok(())
        }
    };
    for variant in &one_of.variants {
        let target = match schemas.by_name.get(&variant.inner) {
            Some(t) => t,
            // Missing references are already caught by
            // `first_unresolved_in_schema`; defensive `continue` so this
            // helper does not double-report.
            None => continue,
        };
        match target {
            ModelSchema::Object(_) => {}
            ModelSchema::StringEnum(_) | ModelSchema::OneOf(_) | ModelSchema::Scalar(_) => {
                return Err(Error::OneOfVariantInnerNotStruct {
                    schema: one_of.name.as_str().to_string(),
                    variant: variant.wire_name.clone(),
                    inner: variant.inner.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Walks a single registered [`ModelSchema`] for references and returns
/// the first one whose target is not registered in `schemas`.
///
/// Variants that carry no references (e.g. string enums, when added) walk
/// nothing and yield `None`.
fn first_unresolved_in_schema<'a>(
    schema: &'a ModelSchema,
    schemas: &Schemas,
) -> Option<&'a SchemaName> {
    match schema {
        ModelSchema::Object(object) => {
            for property in object.properties.values() {
                if let Some(missing) = first_unresolved_reference(&property.ty, schemas) {
                    return Some(missing);
                }
            }
            None
        }
        // String-enum schemas carry no property references; nothing to walk.
        ModelSchema::StringEnum(_) => None,
        ModelSchema::OneOf(one_of) => {
            for variant in &one_of.variants {
                if !schemas.by_name.contains_key(&variant.inner) {
                    return Some(&variant.inner);
                }
            }
            None
        }
        // Scalar schemas carry a single leaf property type and no
        // references; nothing to walk. In practice scalar schemas are
        // filtered out before reaching this point (they are never
        // registered under `#/components/schemas`), but the arm is
        // exhaustive for defensive correctness.
        ModelSchema::Scalar(_) => None,
    }
}

/// Returns the first [`PropertyType::Reference`] encountered in `ty`
/// whose name is not registered in `schemas`, walking
/// [`PropertyType::Array`] and [`PropertyType::Nullable`].
///
/// A reference whose name matches one of the eight primitive scalar
/// names ([`primitive_property_type_for`]) is treated as resolved even
/// when no such entry is registered: primitives implement `Schema` (so
/// they can appear as generic arguments) but not `IsRegistrable`, so
/// `Schemas::add::<i64>()` is intentionally rejected at compile time
/// and the name is never in `schemas.by_name`. The boundary conversion
/// inlines the leaf scalar shape at the reference position, so the
/// resulting OAS document has no dangling `$ref` to follow.
fn first_unresolved_reference<'a>(
    ty: &'a PropertyType,
    schemas: &Schemas,
) -> Option<&'a SchemaName> {
    match ty {
        PropertyType::Reference(name) => {
            if schemas.by_name.contains_key(name) || primitive_property_type_for(name).is_some() {
                None
            } else {
                Some(name)
            }
        }
        PropertyType::Array(inner) | PropertyType::Nullable(inner) => {
            first_unresolved_reference(inner, schemas)
        }
        PropertyType::Int32
        | PropertyType::Int64
        | PropertyType::UInt32
        | PropertyType::UInt64
        | PropertyType::Float
        | PropertyType::Double
        | PropertyType::String
        | PropertyType::Boolean => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;
    use frieze_model::{Error, Presence, Property, PropertyType, SchemaName};

    /// `DummyUser` and `DummyProfile` deliberately omit a `register_into`
    /// override: the default trait impl pushes only `Self`, so these
    /// hand-written `impl Schema`s exercise the non-recursive default
    /// path and let us assert the low-level behaviour (silent dedup,
    /// unresolved-reference detection) without depending on the derive.
    struct DummyUser;

    impl Schema for DummyUser {
        fn name() -> String {
            "User".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "User",
                vec![
                    Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                    Property::new("name", PropertyType::String, Presence::Required).unwrap(),
                ],
            )
            .unwrap()
        }
    }
    impl IsRegistrable for DummyUser {}

    #[test]
    fn build_dedups_same_root_silently() {
        // Two `add::<DummyUser>()` calls used to surface
        // `Error::DuplicateSchema`; with transitive `register_into`
        // semantics the same root being reached twice is normal (e.g. a
        // recursive type, or two siblings referencing the same nested
        // struct), so the builder silently keeps one entry per name.
        let schemas = SchemasBuilder::new()
            .add::<DummyUser>()
            .add::<DummyUser>()
            .build()
            .expect("duplicate adds collapse silently to a single entry");
        assert_eq!(schemas.by_name.len(), 1);
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("User").unwrap()));
    }

    struct DummyProfile;

    impl Schema for DummyProfile {
        fn name() -> String {
            "Profile".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "Profile",
                vec![Property::new(
                    "user",
                    PropertyType::Reference(SchemaName::new("User").unwrap()),
                    Presence::Required,
                )
                .unwrap()],
            )
            .unwrap()
        }
    }
    impl IsRegistrable for DummyProfile {}

    #[test]
    fn build_resolves_explicit_reference() {
        let schemas = SchemasBuilder::new()
            .add::<DummyProfile>()
            .add::<DummyUser>()
            .build()
            .expect("explicit registration resolves the reference");
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("User").unwrap()));
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("Profile").unwrap()));
    }

    #[test]
    fn build_detects_unresolved_reference_for_manual_impl() {
        // `DummyProfile` keeps the default (non-recursive)
        // `register_into`, so adding only the profile does not pull in
        // `DummyUser`. The reference `Profile.user -> User` therefore
        // dangles and the builder fails fast.
        //
        // This exercises the path where `UnresolvedReference` is still
        // raised: hand-written `impl Schema`s that reference other types
        // but do not override `register_into` to walk their dependencies.
        // Code using `#[derive(Schema)]` never lands here because the
        // derived `register_into` walks each field type.
        let err = SchemasBuilder::new()
            .add::<DummyProfile>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::UnresolvedReference(SchemaName::new("User").unwrap())
        );
    }
}
