//! A validated collection of [`Schema`]s.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::schema::Schema;
use crate::schema_name::SchemaName;

/// A validated collection of [`Schema`]s.
///
/// Schemas are stored in a [`BTreeMap`] so that top-level keys are emitted in
/// alphabetical order, matching the documented output ordering.
///
/// Validation happens once, in [`Schemas::new`]. The `by_name` field is `pub`
/// because the type's contract is its shape, not behavior: callers iterate,
/// look up, count, etc. directly through the inner map's API. Maintaining the
/// documented invariants on a value built via struct-literal or
/// post-construction mutation is the caller's responsibility — the
/// constructor is the only place that checks them.
#[derive(Debug, Clone, Default)]
pub struct Schemas {
    pub by_name: BTreeMap<SchemaName, Schema>,
}

impl Schemas {
    /// Builds a collection from a sequence of schemas, rejecting duplicate
    /// schema names.
    pub fn new(schemas: Vec<Schema>) -> Result<Self, Error> {
        let mut by_name: BTreeMap<SchemaName, Schema> = BTreeMap::new();
        for schema in schemas {
            let key = schema.name.clone();
            if by_name.contains_key(&key) {
                return Err(Error::DuplicateSchema(key.into_string()));
            }
            by_name.insert(key, schema);
        }
        Ok(Self { by_name })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::Property;
    use crate::property_type::PropertyType;

    fn user_schema() -> Schema {
        Schema::new(
            "User",
            vec![Property::new("id", PropertyType::Int64, false).unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn rejects_duplicate_names() {
        let err = Schemas::new(vec![user_schema(), user_schema()]).unwrap_err();
        assert_eq!(err, Error::DuplicateSchema("User".into()));
    }

    #[test]
    fn iterates_alphabetically() {
        let a = Schema::new(
            "Album",
            vec![Property::new("id", PropertyType::Int64, false).unwrap()],
        )
        .unwrap();
        let u = user_schema();
        let schemas = Schemas::new(vec![u, a]).unwrap();
        let names: Vec<&str> = schemas.by_name.keys().map(|n| n.as_str()).collect();
        assert_eq!(names, vec!["Album", "User"]);
    }
}
