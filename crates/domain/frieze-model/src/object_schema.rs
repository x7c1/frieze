//! A validated object schema: a non-empty name plus at least one property.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::description::normalize_description;
use crate::error::Error;
use crate::property::Property;
use crate::property_name::PropertyName;
use crate::schema_name::SchemaName;

/// A standard ("object-typed") schema in its validated form: a non-empty
/// name plus at least one property, with no duplicate property names.
///
/// Properties are stored in declaration order (the order passed to
/// [`ObjectSchema::new`]).
///
/// Validation happens once, in [`ObjectSchema::new`]. The fields are
/// `pub` because the type's contract is its shape, not behavior:
/// callers may read or (re-)assign fields directly. Maintaining the
/// documented invariants on a value built via struct-literal or
/// post-construction mutation is the caller's responsibility — the
/// constructor is the only place that checks them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSchema {
    pub name: SchemaName,
    pub properties: IndexMap<PropertyName, Property>,
    /// Free-form description text sourced from the originating Rust
    /// `///` doc-comment on the struct. Empty / whitespace-only inputs
    /// are normalized to `None` at the [`ObjectSchema::with_description`]
    /// entry point.
    pub description: Option<String>,
}

impl ObjectSchema {
    /// Builds an object schema, rejecting empty names, empty property
    /// lists, and duplicate property names. The description is
    /// initialized to `None`; use [`ObjectSchema::with_description`] to
    /// attach one.
    pub fn new(name: impl Into<String>, properties: Vec<Property>) -> Result<Self, Error> {
        let name = SchemaName::new(name)?;
        if properties.is_empty() {
            return Err(Error::NoProperties(name.into_string()));
        }
        let mut map: IndexMap<PropertyName, Property> = IndexMap::with_capacity(properties.len());
        for property in properties {
            let key = property.name.clone();
            if map.contains_key(&key) {
                return Err(Error::DuplicateProperty {
                    schema: name.into_string(),
                    property: key.as_str().to_string(),
                });
            }
            map.insert(key, property);
        }
        Ok(Self {
            name,
            properties: map,
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
    use crate::presence::Presence;
    use crate::property_type::PropertyType;

    #[test]
    fn rejects_empty_name() {
        let property = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let err = ObjectSchema::new("", vec![property]).unwrap_err();
        assert_eq!(err, Error::EmptySchemaName);
    }

    #[test]
    fn rejects_no_properties() {
        let err = ObjectSchema::new("User", vec![]).unwrap_err();
        assert_eq!(err, Error::NoProperties("User".into()));
    }

    #[test]
    fn rejects_duplicate_properties() {
        let a = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let b = Property::new("id", PropertyType::String, Presence::Required).unwrap();
        let err = ObjectSchema::new("User", vec![a, b]).unwrap_err();
        assert_eq!(
            err,
            Error::DuplicateProperty {
                schema: "User".into(),
                property: "id".into()
            }
        );
    }

    #[test]
    fn preserves_declaration_order() {
        let id = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let name = Property::new("name", PropertyType::String, Presence::Required).unwrap();
        let schema = ObjectSchema::new("User", vec![id, name]).unwrap();
        let keys: Vec<&str> = schema.properties.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["id", "name"]);
    }

    #[test]
    fn description_is_none_by_default() {
        let id = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let schema = ObjectSchema::new("User", vec![id]).unwrap();
        assert_eq!(schema.description, None);
    }

    #[test]
    fn with_description_attaches_text() {
        let id = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let schema = ObjectSchema::new("User", vec![id])
            .unwrap()
            .with_description(Some("a registered user".into()));
        assert_eq!(schema.description.as_deref(), Some("a registered user"));
    }

    #[test]
    fn with_description_normalizes_blank_to_none() {
        let id = Property::new("id", PropertyType::Int64, Presence::Required).unwrap();
        let schema = ObjectSchema::new("User", vec![id])
            .unwrap()
            .with_description(Some("   \n  ".into()));
        assert_eq!(schema.description, None);
    }
}
