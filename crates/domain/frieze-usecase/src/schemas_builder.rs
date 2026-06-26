//! Builder that collects [`Schema`] implementations into a validated
//! [`frieze_model::Schemas`].

use frieze_model::{Error, Schemas};

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

    /// Finalizes the collection, checking for duplicate schema names.
    pub fn build(self) -> Result<Schemas, Error> {
        Schemas::new(self.schemas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frieze_model::{Error, Presence, Property, PropertyType};

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
        assert_eq!(err, Error::DuplicateSchema("User".into()));
    }
}
