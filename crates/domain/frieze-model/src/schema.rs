//! A validated schema with a non-empty name and at least one property.

use indexmap::IndexMap;

use crate::error::Error;
use crate::property::Property;
use crate::property_name::PropertyName;
use crate::schema_name::SchemaName;

/// A schema in its validated form: a non-empty name plus at least one property,
/// with no duplicate property names.
///
/// Properties are stored in declaration order (the order passed to
/// [`Schema::new`]).
///
/// Validation happens once, in [`Schema::new`]. The fields are `pub` because
/// the type's contract is its shape, not behavior: callers may read or
/// (re-)assign fields directly. Maintaining the documented invariants on a
/// value built via struct-literal or post-construction mutation is the
/// caller's responsibility — the constructor is the only place that checks
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub name: SchemaName,
    pub properties: IndexMap<PropertyName, Property>,
}

impl Schema {
    /// Builds a schema, rejecting empty names, empty property lists, and
    /// duplicate property names.
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_type::PropertyType;

    #[test]
    fn rejects_empty_name() {
        let property = Property::new("id", PropertyType::Int64, false).unwrap();
        let err = Schema::new("", vec![property]).unwrap_err();
        assert_eq!(err, Error::EmptySchemaName);
    }

    #[test]
    fn rejects_no_properties() {
        let err = Schema::new("User", vec![]).unwrap_err();
        assert_eq!(err, Error::NoProperties("User".into()));
    }

    #[test]
    fn rejects_duplicate_properties() {
        let a = Property::new("id", PropertyType::Int64, false).unwrap();
        let b = Property::new("id", PropertyType::String, false).unwrap();
        let err = Schema::new("User", vec![a, b]).unwrap_err();
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
        let id = Property::new("id", PropertyType::Int64, false).unwrap();
        let name = Property::new("name", PropertyType::String, false).unwrap();
        let schema = Schema::new("User", vec![id, name]).unwrap();
        let keys: Vec<&str> = schema.properties.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["id", "name"]);
    }
}
