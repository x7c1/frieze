//! Domain types whose invariants are enforced by the type system.
//!
//! Types here use private fields and `pub fn new(...)` constructors so they
//! cannot be built via struct literals from outside the crate.

use indexmap::IndexMap;
use thiserror::Error;

/// Errors that can occur while constructing domain types.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("schema name must not be empty")]
    EmptySchemaName,
    #[error("property name must not be empty")]
    EmptyPropertyName,
    #[error("schema `{0}` has no properties")]
    NoProperties(String),
    #[error("schema `{schema}` declares duplicate property `{property}`")]
    DuplicateProperty { schema: String, property: String },
}

/// Primitive scalar types currently supported by the derive in Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    /// Maps to OpenAPI `type: integer, format: int64`.
    Int64,
    /// Maps to OpenAPI `type: string` (no format).
    String,
}

/// A validated property attached to a schema.
///
/// Constructed via [`ValidatedProperty::new`]; the inner fields are private to
/// prevent construction that bypasses validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedProperty {
    name: String,
    ty: PropertyType,
}

impl ValidatedProperty {
    /// Builds a property, rejecting empty names.
    pub fn new(name: impl Into<String>, ty: PropertyType) -> Result<Self, ModelError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ModelError::EmptyPropertyName);
        }
        Ok(Self { name, ty })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> PropertyType {
        self.ty
    }
}

/// A validated schema with at least one property and a non-empty name.
///
/// Properties are stored in declaration order (the order passed to
/// [`ValidatedSchema::new`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSchema {
    name: String,
    properties: IndexMap<String, ValidatedProperty>,
}

impl ValidatedSchema {
    /// Builds a schema, rejecting empty names, empty property lists, and
    /// duplicate property names.
    pub fn new(
        name: impl Into<String>,
        properties: Vec<ValidatedProperty>,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ModelError::EmptySchemaName);
        }
        if properties.is_empty() {
            return Err(ModelError::NoProperties(name));
        }
        let mut map: IndexMap<String, ValidatedProperty> =
            IndexMap::with_capacity(properties.len());
        for property in properties {
            let key = property.name().to_string();
            if map.contains_key(&key) {
                return Err(ModelError::DuplicateProperty {
                    schema: name,
                    property: key,
                });
            }
            map.insert(key, property);
        }
        Ok(Self {
            name,
            properties: map,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Properties in declaration order.
    pub fn properties(&self) -> &IndexMap<String, ValidatedProperty> {
        &self.properties
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_rejects_empty_name() {
        let err = ValidatedProperty::new("", PropertyType::Int64).unwrap_err();
        assert_eq!(err, ModelError::EmptyPropertyName);
    }

    #[test]
    fn schema_rejects_empty_name() {
        let property = ValidatedProperty::new("id", PropertyType::Int64).unwrap();
        let err = ValidatedSchema::new("", vec![property]).unwrap_err();
        assert_eq!(err, ModelError::EmptySchemaName);
    }

    #[test]
    fn schema_rejects_no_properties() {
        let err = ValidatedSchema::new("User", vec![]).unwrap_err();
        assert_eq!(err, ModelError::NoProperties("User".into()));
    }

    #[test]
    fn schema_rejects_duplicate_properties() {
        let a = ValidatedProperty::new("id", PropertyType::Int64).unwrap();
        let b = ValidatedProperty::new("id", PropertyType::String).unwrap();
        let err = ValidatedSchema::new("User", vec![a, b]).unwrap_err();
        assert_eq!(
            err,
            ModelError::DuplicateProperty {
                schema: "User".into(),
                property: "id".into()
            }
        );
    }

    #[test]
    fn schema_preserves_declaration_order() {
        let id = ValidatedProperty::new("id", PropertyType::Int64).unwrap();
        let name = ValidatedProperty::new("name", PropertyType::String).unwrap();
        let schema = ValidatedSchema::new("User", vec![id, name]).unwrap();
        let keys: Vec<&str> = schema.properties().keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["id", "name"]);
    }
}
