//! Builder that collects [`Schema`] implementations into a validated
//! [`frieze_model::Schemas`].

use frieze_model::{Error, PropertyType, SchemaName, Schemas};

use crate::schema::Schema;

/// In-progress collection of schemas.
#[derive(Debug, Default)]
pub struct SchemasBuilder {
    schemas: Vec<frieze_model::Schema>,
}

impl SchemasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()`.
    pub fn add<T: Schema>(mut self) -> Self {
        self.schemas.push(T::schema());
        self
    }

    /// Finalizes the collection, checking for duplicate schema names
    /// **and** that every `$ref` resolves to a registered schema.
    ///
    /// References are gathered by walking each property's type tree
    /// (recursing into `Array(...)` and `Nullable(...)`). The first ref
    /// that points at a schema not in the collection produces
    /// [`Error::UnresolvedReference`], in declaration order — the same
    /// fail-fast UX as [`Error::DuplicateSchema`].
    pub fn build(self) -> Result<Schemas, Error> {
        let schemas = Schemas::new(self.schemas)?;
        for schema in schemas.by_name.values() {
            for property in schema.properties.values() {
                if let Some(missing) = first_unresolved_reference(&property.ty, &schemas) {
                    return Err(Error::UnresolvedReference(missing.clone()));
                }
            }
        }
        Ok(schemas)
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
    use frieze_model::{Error, Presence, Property, PropertyType, SchemaName};

    struct DummyUser;

    impl Schema for DummyUser {
        fn name() -> &'static str {
            "User"
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new(
                "User",
                vec![
                    Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                    Property::new("name", PropertyType::String, Presence::Required).unwrap(),
                ],
            )
            .unwrap()
        }
    }

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
        fn name() -> &'static str {
            "Profile"
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new(
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
