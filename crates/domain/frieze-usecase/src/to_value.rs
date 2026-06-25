//! Boundary conversion from [`frieze_model::Schemas`] to
//! [`serde_yaml::Value`], via [`frieze_openapi`].

use frieze_model::{PropertyType, Schema, Schemas};
use frieze_openapi::{SchemaObject, SchemaType};
use indexmap::IndexMap;
use serde_yaml::{Mapping, Value};

/// Converts a validated [`Schemas`] collection to a [`serde_yaml::Value`],
/// preserving the canonical output order:
///
/// - top-level keys (schema names): alphabetical (via [`std::collections::BTreeMap`])
/// - inside one schema: `type`, `properties`, `required`
/// - `properties`: declaration order (via [`IndexMap`])
/// - `required`: same order as `properties`
pub fn to_value(schemas: &Schemas) -> Value {
    let mut top = Mapping::new();
    for (name, schema) in &schemas.by_name {
        let openapi = to_openapi(schema);
        top.insert(
            Value::String(name.as_str().to_string()),
            schema_object_to_value(&openapi),
        );
    }
    Value::Mapping(top)
}

/// Boundary conversion: validated domain schema -> plain OAS schema object.
fn to_openapi(schema: &Schema) -> SchemaObject {
    let mut properties: IndexMap<String, SchemaObject> = IndexMap::new();
    let mut required: Vec<String> = Vec::with_capacity(schema.properties.len());
    for (name, property) in &schema.properties {
        let (ty, format) = match property.ty {
            PropertyType::Int64 => (SchemaType::Integer, Some("int64".to_string())),
            PropertyType::String => (SchemaType::String, None),
            PropertyType::Boolean => (SchemaType::Boolean, None),
        };
        properties.insert(
            name.as_str().to_string(),
            SchemaObject {
                ty: Some(ty),
                format,
                properties: None,
                required: None,
            },
        );
        required.push(name.as_str().to_string());
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
            SchemaType::Boolean => "boolean",
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
    use crate::schema::Schema as SchemaTrait;
    use crate::schemas_builder::SchemasBuilder;
    use frieze_model::{Property, PropertyType};

    struct DummyUser;

    impl SchemaTrait for DummyUser {
        fn name() -> &'static str {
            "User"
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new(
                "User",
                vec![
                    Property::new("id", PropertyType::Int64).unwrap(),
                    Property::new("name", PropertyType::String).unwrap(),
                ],
            )
            .unwrap()
        }
    }

    #[test]
    fn preserves_property_order() {
        let schemas = SchemasBuilder::new().add::<DummyUser>().build().unwrap();
        let value = to_value(&schemas);
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
