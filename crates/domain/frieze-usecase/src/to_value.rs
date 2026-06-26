//! Boundary conversion from [`frieze_model::Schemas`] to
//! [`serde_yaml::Value`], via [`frieze_openapi`].

use frieze_model::{PropertyType, Schema, Schemas};
use frieze_openapi::{SchemaObject, SchemaType};
use indexmap::IndexMap;
use serde_yaml::{Mapping, Number, Value};

/// Converts a validated [`Schemas`] collection to a [`serde_yaml::Value`],
/// preserving the canonical output order:
///
/// - top-level keys (schema names): alphabetical (via [`std::collections::BTreeMap`])
/// - inside one schema: `type`, `format`, `minimum`, `properties`, `required`
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
        properties.insert(name.as_str().to_string(), property_to_openapi(property.ty));
        required.push(name.as_str().to_string());
    }
    SchemaObject {
        ty: Some(SchemaType::Object),
        format: None,
        minimum: None,
        properties: Some(properties),
        required: Some(required),
    }
}

/// Single mapping from a [`PropertyType`] to the OAS schema object that
/// represents it. This is the one place to edit when a new scalar variant is
/// added to [`PropertyType`].
fn property_to_openapi(pt: PropertyType) -> SchemaObject {
    let (ty, format, minimum) = match pt {
        PropertyType::Int32 => (SchemaType::Integer, Some("int32"), None),
        PropertyType::Int64 => (SchemaType::Integer, Some("int64"), None),
        PropertyType::UInt32 => (SchemaType::Integer, Some("int32"), Some(0.0)),
        PropertyType::UInt64 => (SchemaType::Integer, Some("int64"), Some(0.0)),
        PropertyType::Float => (SchemaType::Number, Some("float"), None),
        PropertyType::Double => (SchemaType::Number, Some("double"), None),
        PropertyType::String => (SchemaType::String, None, None),
        PropertyType::Boolean => (SchemaType::Boolean, None, None),
    };
    SchemaObject {
        ty: Some(ty),
        format: format.map(str::to_owned),
        minimum,
        properties: None,
        required: None,
    }
}

/// Serializes a [`SchemaObject`] into a [`Value`] with manually ordered keys
/// (`type`, `format`, `minimum`, `properties`, `required`) so that the YAML
/// output is stable.
fn schema_object_to_value(schema: &SchemaObject) -> Value {
    let mut map = Mapping::new();
    if let Some(ty) = schema.ty {
        map.insert(Value::String("type".into()), schema_type_to_value(ty));
    }
    if let Some(format) = &schema.format {
        map.insert(
            Value::String("format".into()),
            Value::String(format.clone()),
        );
    }
    if let Some(minimum) = schema.minimum {
        map.insert(Value::String("minimum".into()), minimum_to_value(minimum));
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

/// Delegates the [`SchemaType`] -> string conversion to its `Serialize`
/// impl (`#[serde(rename_all = "lowercase")]`), so adding a new variant no
/// longer requires touching this module.
fn schema_type_to_value(ty: SchemaType) -> Value {
    serde_yaml::to_value(ty)
        .expect("frieze: serializing a fieldless SchemaType variant to Value cannot fail")
}

/// Emits a `minimum` value as an integer when the bound is a whole number
/// that round-trips losslessly through `i64` (the OAS-idiomatic
/// `minimum: 0` rather than `0.0`), and as a float otherwise.
///
/// `SchemaObject.minimum` is typed as `f64` so the API can carry fractional
/// bounds in the future, but the only values Phase 1 produces are integer
/// constants (0 for `u32` / `u64`), which should render as integers.
fn minimum_to_value(minimum: f64) -> Value {
    let as_int = minimum as i64;
    if minimum.is_finite() && (as_int as f64) == minimum {
        Value::Number(Number::from(as_int))
    } else {
        Value::Number(Number::from(minimum))
    }
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
