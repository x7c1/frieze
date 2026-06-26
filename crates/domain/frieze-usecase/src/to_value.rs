//! Boundary conversion from [`frieze_model::Schemas`] to
//! [`serde_yaml::Value`], via [`frieze_openapi`].

use frieze_model::{Property, PropertyType, Schema, SchemaName, Schemas};
use frieze_openapi::{SchemaObject, SchemaType};
use indexmap::IndexMap;
use serde_yaml::{Mapping, Number, Value};

/// Converts a validated [`Schemas`] collection to a [`serde_yaml::Value`],
/// preserving the canonical output order:
///
/// - top-level keys (schema names): alphabetical (via [`std::collections::BTreeMap`])
/// - inside one schema: `$ref`, `type`, `items`, `format`, `minimum`,
///   `allOf`, `oneOf`, (`nullable` under `oas-3-0`,) `properties`,
///   `required`
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
///
/// The `required` array is built from each property's [`Property::presence`]
/// — and only that. Value-level nullability lives on the type tree
/// ([`PropertyType::Nullable`]) and is rendered independently by
/// [`property_to_openapi`].
fn to_openapi(schema: &Schema) -> SchemaObject {
    let mut properties: IndexMap<String, SchemaObject> = IndexMap::new();
    let mut required: Vec<String> = Vec::with_capacity(schema.properties.len());
    for (name, property) in &schema.properties {
        properties.insert(name.as_str().to_string(), property_to_openapi(property));
        if property.presence.is_required() {
            required.push(name.as_str().to_string());
        }
    }
    SchemaObject {
        ty: Some(SchemaType::Object),
        properties: Some(properties),
        required: Some(required),
        ..SchemaObject::empty()
    }
}

/// Single mapping from a [`Property`] to the OAS schema object that
/// represents it. The property's presence is consumed up at
/// [`to_openapi`] (for the `required` array); only the type tree affects
/// the per-property schema object emitted here.
fn property_to_openapi(property: &Property) -> SchemaObject {
    property_type_to_openapi(&property.ty)
}

/// Single mapping from a [`PropertyType`] to the matching schema object.
///
/// Recurses on [`PropertyType::Array`] so the element schema is rendered
/// into the `items` slot, and on [`PropertyType::Nullable`] so the
/// nullability marker is attached at whichever position in the tree it
/// appears. `Array(Nullable(...))` therefore makes the items nullable;
/// `Nullable(Array(...))` makes the array itself nullable.
///
/// [`PropertyType::Reference`] is rendered as a pure `$ref` schema; if
/// the renderer's recursion finds itself wrapping a `Reference` in
/// [`PropertyType::Nullable`], it falls through to the OAS-version-specific
/// "nullable reference" shape ([`nullable_reference_object`]) rather than
/// attempting to attach `nullable: true` directly to the `$ref` (which is
/// invalid OAS).
fn property_type_to_openapi(ty: &PropertyType) -> SchemaObject {
    let (schema_ty, format, minimum) = match ty {
        PropertyType::Int32 => (SchemaType::Integer, Some("int32"), None),
        PropertyType::Int64 => (SchemaType::Integer, Some("int64"), None),
        PropertyType::UInt32 => (SchemaType::Integer, Some("int32"), Some(0.0)),
        PropertyType::UInt64 => (SchemaType::Integer, Some("int64"), Some(0.0)),
        PropertyType::Float => (SchemaType::Number, Some("float"), None),
        PropertyType::Double => (SchemaType::Number, Some("double"), None),
        PropertyType::String => (SchemaType::String, None, None),
        PropertyType::Boolean => (SchemaType::Boolean, None, None),
        PropertyType::Array(inner) => {
            return SchemaObject {
                ty: Some(SchemaType::Array),
                items: Some(Box::new(property_type_to_openapi(inner))),
                ..SchemaObject::empty()
            };
        }
        PropertyType::Nullable(inner) => {
            // A nullable reference cannot simply set `nullable: true` on a
            // `$ref` schema object — `$ref` siblings are ignored in OAS 3.0
            // and disallowed in 3.1. Use the version-specific wrap instead.
            if let PropertyType::Reference(name) = inner.as_ref() {
                return nullable_reference_object(name);
            }
            let mut inner_schema = property_type_to_openapi(inner);
            inner_schema.nullable = Some(true);
            return inner_schema;
        }
        PropertyType::Reference(name) => {
            return reference_object(name);
        }
    };
    SchemaObject {
        ty: Some(schema_ty),
        format: format.map(str::to_owned),
        minimum,
        ..SchemaObject::empty()
    }
}

/// Builds a pure `$ref` schema object pointing at
/// `#/components/schemas/<name>`.
fn reference_object(name: &SchemaName) -> SchemaObject {
    SchemaObject {
        reference: Some(format!("#/components/schemas/{}", name.as_str())),
        ..SchemaObject::empty()
    }
}

/// Builds the OAS-version-specific "nullable reference" schema object.
///
/// - OAS 3.0: `allOf: [{$ref}], nullable: true`. The `allOf` wrap is the
///   idiomatic 3.0 escape hatch for siblings on a referencing schema.
/// - OAS 3.1: `oneOf: [{$ref}, {type: "null"}]`. The `nullable` keyword
///   was dropped in 3.1, and a sibling `$ref` would be rejected.
///
/// The `$ref` element is always first; the null sibling is second. This
/// is purely for snapshot stability.
#[cfg(feature = "oas-3-0")]
fn nullable_reference_object(name: &SchemaName) -> SchemaObject {
    SchemaObject {
        all_of: Some(vec![reference_object(name)]),
        nullable: Some(true),
        ..SchemaObject::empty()
    }
}

#[cfg(feature = "oas-3-1")]
fn nullable_reference_object(name: &SchemaName) -> SchemaObject {
    SchemaObject {
        one_of: Some(vec![
            reference_object(name),
            SchemaObject {
                // `type: "null"` is emitted as a quoted string by the
                // renderer to preserve the user-visible scalar form. See
                // `schema_type_to_value` / `insert_type`.
                ty: Some(SchemaType::Null),
                ..SchemaObject::empty()
            },
        ]),
        ..SchemaObject::empty()
    }
}

/// Serializes a [`SchemaObject`] into a [`Value`] with manually ordered keys
/// so that the YAML output is stable.
///
/// Key ordering depends on the selected OAS version feature:
///
/// - `oas-3-0`: `$ref`, `type`, `items`, `format`, `minimum`, `allOf`,
///   `oneOf`, `nullable`, `properties`, `required`. The nullability
///   intent for scalars is emitted as `nullable: true`; the nullability
///   intent for references is emitted via `allOf` + `nullable`.
/// - `oas-3-1`: `$ref`, `type`, `items`, `format`, `minimum`, `allOf`,
///   `oneOf`, `properties`, `required`. The nullability intent for
///   scalars is folded into `type` as a 2-element sequence
///   `[<base>, "null"]`; the nullability intent for references is
///   emitted via `oneOf` with a `{type: "null"}` sibling. The OAS 3.1
///   spec drops the `nullable` keyword.
///
/// `items` is emitted on array schemas only; the element schema is
/// rendered through the same `schema_object_to_value` recursively so its
/// own keys obey the same ordering rules (and so a nullable item gets
/// its `nullable` marker at the items level, not the array level).
///
/// When `$ref` is present, no other keys are emitted: OAS treats a
/// `$ref` schema as a leaf and discards sibling keys.
fn schema_object_to_value(schema: &SchemaObject) -> Value {
    let mut map = Mapping::new();
    if let Some(reference) = &schema.reference {
        map.insert(
            Value::String("$ref".into()),
            Value::String(reference.clone()),
        );
        return Value::Mapping(map);
    }
    insert_type(&mut map, schema);
    if let Some(items) = &schema.items {
        map.insert(Value::String("items".into()), schema_object_to_value(items));
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
    if let Some(all_of) = &schema.all_of {
        map.insert(
            Value::String("allOf".into()),
            Value::Sequence(all_of.iter().map(schema_object_to_value).collect()),
        );
    }
    if let Some(one_of) = &schema.one_of {
        map.insert(
            Value::String("oneOf".into()),
            Value::Sequence(one_of.iter().map(schema_object_to_value).collect()),
        );
    }
    insert_nullable(&mut map, schema);
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

/// Emits the `type` key.
///
/// Under `oas-3-0`, `type` is always a scalar string (the nullability
/// intent is emitted separately by [`insert_nullable`]).
///
/// Under `oas-3-1`, `type` becomes a 2-element sequence `[<base>, "null"]`
/// when the schema is nullable. The `"null"` is intentionally quoted —
/// unquoted `null` in YAML resolves to the null value, not the string
/// `"null"`.
#[cfg(feature = "oas-3-0")]
fn insert_type(map: &mut Mapping, schema: &SchemaObject) {
    if let Some(ty) = schema.ty {
        map.insert(Value::String("type".into()), schema_type_to_value(ty));
    }
}

#[cfg(feature = "oas-3-1")]
fn insert_type(map: &mut Mapping, schema: &SchemaObject) {
    if let Some(ty) = schema.ty {
        let base = schema_type_to_value(ty);
        let value = if schema.nullable == Some(true) {
            Value::Sequence(vec![base, Value::String("null".into())])
        } else {
            base
        };
        map.insert(Value::String("type".into()), value);
    }
}

/// Emits the nullability marker between `oneOf` and `properties`.
///
/// Under `oas-3-0`, a nullable schema gets `nullable: true`. Under
/// `oas-3-1`, the nullability marker is folded into `type` (see
/// [`insert_type`]) and this function emits nothing.
#[cfg(feature = "oas-3-0")]
fn insert_nullable(map: &mut Mapping, schema: &SchemaObject) {
    if schema.nullable == Some(true) {
        map.insert(Value::String("nullable".into()), Value::Bool(true));
    }
}

#[cfg(feature = "oas-3-1")]
fn insert_nullable(_map: &mut Mapping, _schema: &SchemaObject) {
    // OAS 3.1 has no `nullable` keyword; the intent is encoded in `type`.
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
    use frieze_model::{Presence, Property, PropertyType};

    struct DummyUser;

    impl SchemaTrait for DummyUser {
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
