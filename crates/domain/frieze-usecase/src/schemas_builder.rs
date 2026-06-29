//! Builder that collects [`Schema`] implementations into a validated
//! [`frieze_model::Schemas`].

use frieze_model::{Error, PropertyType, Schema as ModelSchema, SchemaName, Schemas};

use crate::schema::IsRegistrable;

/// In-progress collection of schemas.
#[derive(Debug, Default)]
pub struct SchemasBuilder {
    schemas: Vec<ModelSchema>,
}

impl SchemasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()`.
    ///
    /// `T` must implement [`IsRegistrable`] — this rejects primitive
    /// scalars at compile time (`Schemas::add::<i64>()` fails to
    /// compile), since primitive scalars implement [`Schema`] only so
    /// they can appear as generic arguments and are not standalone OAS
    /// schema entries. `#[derive(Schema)]` emits the `IsRegistrable`
    /// impl for struct and enum inputs.
    pub fn add<T: IsRegistrable>(mut self) -> Self {
        self.schemas.push(T::schema());
        self
    }

    /// Finalizes the collection, checking for duplicate schema names
    /// **and** that every `$ref` resolves to a registered schema.
    ///
    /// References are gathered by walking each property's type tree
    /// (recursing into `Array(...)` and `Nullable(...)`) and each
    /// `oneOf` variant's inner reference. The first ref that points at
    /// a schema not in the collection produces
    /// [`Error::UnresolvedReference`], in declaration order — the same
    /// fail-fast UX as [`Error::DuplicateSchema`].
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
fn first_unresolved_reference<'a>(
    ty: &'a PropertyType,
    schemas: &Schemas,
) -> Option<&'a SchemaName> {
    match ty {
        PropertyType::Reference(name) => {
            if schemas.by_name.contains_key(name) {
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
    fn build_rejects_duplicates() {
        let err = SchemasBuilder::new()
            .add::<DummyUser>()
            .add::<DummyUser>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::DuplicateSchema(SchemaName::new("User").unwrap())
        );
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
    fn build_detects_unresolved_reference() {
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
