//! Use cases for frieze.
//!
//! Defines the [`Schema`] trait that user types implement (typically through
//! the derive macro in `frieze-macros`), the [`Schemas`] builder that collects
//! schemas, and the boundary conversion from `frieze-model` to
//! `frieze-openapi`.

use std::collections::BTreeMap;

use frieze_model::{PropertyType, ValidatedSchema};
use frieze_openapi::{SchemaObject, SchemaType};
use indexmap::IndexMap;
use serde_yaml::{Mapping, Value};
use thiserror::Error;

/// Trait implemented by types that can be expressed as an OpenAPI schema.
///
/// `#[derive(frieze::Schema)]` generates an implementation of this trait.
pub trait Schema {
    /// The schema name used as the key under `#/components/schemas`.
    fn name() -> &'static str;

    /// Builds the validated domain representation of this schema.
    fn schema() -> ValidatedSchema;
}

/// Errors that can occur while building the schema collection.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildError {
    #[error("schema `{0}` was added more than once")]
    DuplicateSchema(String),
}

/// In-progress collection of schemas.
#[derive(Debug, Default)]
pub struct Schemas {
    schemas: Vec<ValidatedSchema>,
}

impl Schemas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()`.
    pub fn add<T: Schema>(mut self) -> Self {
        self.schemas.push(T::schema());
        self
    }

    /// Finalizes the collection, checking for duplicate schema names.
    pub fn build(self) -> Result<BuiltSchemas, BuildError> {
        let mut by_name: BTreeMap<String, ValidatedSchema> = BTreeMap::new();
        for schema in self.schemas {
            let name = schema.name().to_string();
            if by_name.contains_key(&name) {
                return Err(BuildError::DuplicateSchema(name));
            }
            by_name.insert(name, schema);
        }
        Ok(BuiltSchemas { by_name })
    }
}

/// The successful output of [`Schemas::build`].
///
/// Schemas are stored in a [`BTreeMap`] so that top-level keys are emitted in
/// alphabetical order, matching the documented output ordering.
#[derive(Debug)]
pub struct BuiltSchemas {
    by_name: BTreeMap<String, ValidatedSchema>,
}

impl BuiltSchemas {
    /// Converts the held domain schemas to `frieze-openapi` types and then to
    /// `serde_yaml::Value`, preserving the canonical output order:
    ///
    /// - top-level keys (schema names): alphabetical (via [`BTreeMap`])
    /// - inside one schema: `type`, `properties`, `required`
    /// - `properties`: declaration order (via [`IndexMap`])
    /// - `required`: same order as `properties`
    pub fn to_value(&self) -> Value {
        let mut top = Mapping::new();
        for (name, schema) in &self.by_name {
            let openapi = to_openapi(schema);
            top.insert(
                Value::String(name.clone()),
                schema_object_to_value(&openapi),
            );
        }
        Value::Mapping(top)
    }
}

/// Boundary conversion: validated domain schema -> plain OAS schema object.
fn to_openapi(schema: &ValidatedSchema) -> SchemaObject {
    let mut properties: IndexMap<String, SchemaObject> = IndexMap::new();
    let mut required: Vec<String> = Vec::with_capacity(schema.properties().len());
    for (name, property) in schema.properties() {
        let (ty, format) = match property.ty() {
            PropertyType::Int64 => (SchemaType::Integer, Some("int64".to_string())),
            PropertyType::String => (SchemaType::String, None),
        };
        properties.insert(
            name.clone(),
            SchemaObject {
                ty: Some(ty),
                format,
                properties: None,
                required: None,
            },
        );
        required.push(name.clone());
    }
    SchemaObject {
        ty: Some(SchemaType::Object),
        format: None,
        properties: Some(properties),
        required: Some(required),
    }
}

/// Serializes a [`SchemaObject`] into a [`Value`] with manually ordered keys
/// (`type`, `properties`, `required`) so that the YAML output is stable.
fn schema_object_to_value(schema: &SchemaObject) -> Value {
    let mut map = Mapping::new();
    if let Some(ty) = schema.ty {
        let ty_str = match ty {
            SchemaType::Object => "object",
            SchemaType::Integer => "integer",
            SchemaType::String => "string",
        };
        map.insert(Value::String("type".into()), Value::String(ty_str.into()));
    }
    if let Some(format) = &schema.format {
        map.insert(
            Value::String("format".into()),
            Value::String(format.clone()),
        );
    }
    if let Some(properties) = &schema.properties {
        let mut inner = Mapping::new();
        for (name, child) in properties {
            inner.insert(Value::String(name.clone()), schema_object_to_value(child));
        }
        map.insert(Value::String("properties".into()), Value::Mapping(inner));
    }
    if let Some(required) = &schema.required {
        let items: Vec<Value> = required
            .iter()
            .map(|name| Value::String(name.clone()))
            .collect();
        map.insert(Value::String("required".into()), Value::Sequence(items));
    }
    Value::Mapping(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frieze_model::{PropertyType, ValidatedProperty, ValidatedSchema};

    struct DummyUser;

    impl Schema for DummyUser {
        fn name() -> &'static str {
            "User"
        }
        fn schema() -> ValidatedSchema {
            ValidatedSchema::new(
                "User",
                vec![
                    ValidatedProperty::new("id", PropertyType::Int64).unwrap(),
                    ValidatedProperty::new("name", PropertyType::String).unwrap(),
                ],
            )
            .unwrap()
        }
    }

    #[test]
    fn build_rejects_duplicates() {
        let err = Schemas::new()
            .add::<DummyUser>()
            .add::<DummyUser>()
            .build()
            .unwrap_err();
        assert_eq!(err, BuildError::DuplicateSchema("User".into()));
    }

    #[test]
    fn to_value_preserves_property_order() {
        let built = Schemas::new().add::<DummyUser>().build().unwrap();
        let value = built.to_value();
        let top = value.as_mapping().unwrap();
        let user = top
            .get(Value::String("User".into()))
            .and_then(Value::as_mapping)
            .unwrap();
        let keys: Vec<&str> = user.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["type", "properties", "required"]);

        let properties = user
            .get(Value::String("properties".into()))
            .and_then(Value::as_mapping)
            .unwrap();
        let property_keys: Vec<&str> = properties.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(property_keys, vec!["id", "name"]);
    }
}
