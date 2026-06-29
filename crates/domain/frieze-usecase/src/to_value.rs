//! Boundary conversion from [`frieze_model::Schemas`] to
//! [`serde_yaml::Value`], via [`frieze_openapi`].

use frieze_model::{Property, PropertyType, Schema, SchemaName, Schemas};
use frieze_openapi::{
    ObjectSchema, OneOfSchema, OneOfVariant, SchemaObject, SchemaType, StringEnumSchema,
};
use indexmap::IndexMap;
use serde_yaml::{Mapping, Number, Value};

/// Converts a validated [`Schemas`] collection to a [`serde_yaml::Value`],
/// preserving the canonical output order:
///
/// - top-level keys (schema names): alphabetical (via [`std::collections::BTreeMap`])
/// - inside one schema: `$ref`, `type`, `description`, `format`,
///   `minimum`, `items`, `required`, `properties`, `allOf`, `oneOf`,
///   (`nullable` under `oas-3-0`,) `enum`
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
/// Each [`Schema`] variant maps to the matching [`SchemaObject`] variant.
/// For [`Schema::Object`], the `required` array is built from each
/// property's [`Property::presence`] — and only that. Value-level
/// nullability lives on the type tree ([`PropertyType::Nullable`]) and is
/// rendered independently by [`property_type_to_object_schema`].
fn to_openapi(schema: &Schema) -> SchemaObject {
    match schema {
        Schema::Object(object) => SchemaObject::Object(object_schema_from_model(object)),
        Schema::StringEnum(string_enum) => SchemaObject::StringEnum(
            StringEnumSchema::new(string_enum.values.clone())
                .with_description(string_enum.description.clone()),
        ),
        Schema::OneOf(one_of) => SchemaObject::OneOf(one_of_from_model(one_of)),
    }
}

/// Boundary conversion for a [`frieze_model::OneOfSchema`].
///
/// The macro side has already composed any per-variant docs into the
/// enum-level description (mirroring the unit-variant enum's behaviour),
/// so the rendering side only consumes the variant's `wire_name` and
/// `inner` reference. The `inner_reference` is pre-formatted here so the
/// emitter does not need to know the JSON-pointer convention.
fn one_of_from_model(one_of: &frieze_model::OneOfSchema) -> OneOfSchema {
    let variants: Vec<OneOfVariant> = one_of
        .variants
        .iter()
        .map(|variant| OneOfVariant {
            wire_name: variant.wire_name.clone(),
            inner_reference: format!("#/components/schemas/{}", variant.inner.as_str()),
        })
        .collect();
    OneOfSchema::new(one_of.tag.clone(), variants).with_description(one_of.description.clone())
}

fn object_schema_from_model(schema: &frieze_model::ObjectSchema) -> ObjectSchema {
    let mut properties: IndexMap<String, ObjectSchema> = IndexMap::new();
    let mut required: Vec<String> = Vec::with_capacity(schema.properties.len());
    for (name, property) in &schema.properties {
        properties.insert(
            name.as_str().to_string(),
            property_to_object_schema(property),
        );
        if property.presence.is_required() {
            required.push(name.as_str().to_string());
        }
    }
    ObjectSchema {
        ty: Some(SchemaType::Object),
        description: schema.description.clone(),
        properties: Some(properties),
        required,
        ..ObjectSchema::empty()
    }
}

/// Single mapping from a [`Property`] to the OAS object schema that
/// represents it. The property's presence is consumed up at
/// [`object_schema_from_model`] (for the `required` array); the property
/// description is attached here at whatever position the type tree
/// dictates — for plain scalar / array properties this is the sibling
/// `description` key on the emitted schema; for a `Reference`-typed
/// property the wrap rules in [`property_type_to_object_schema`] place
/// the description on the outer wrapper.
fn property_to_object_schema(property: &Property) -> ObjectSchema {
    attach_description(
        property_type_to_object_schema(&property.ty),
        property.description.as_deref(),
    )
}

/// Attaches a property-level description to the object schema returned
/// by [`property_type_to_object_schema`], honouring the four-case
/// `(description, nullable)` matrix against `$ref` siblings:
///
/// |              | not nullable                 | nullable                                  |
/// |--------------|------------------------------|-------------------------------------------|
/// | no descr.    | bare `$ref` (no change)      | `{allOf: [{$ref}], nullable: true}` (3.0) / `{oneOf: [{$ref}, {null}]}` (3.1) |
/// | with descr.  | OAS 3.0: `{description, allOf: [{$ref}]}` / OAS 3.1: `{$ref, description}` | description attaches to the existing wrap |
///
/// The "nullable" cases hand us a schema whose `reference` is already
/// `None` (the reference now lives inside `all_of` / `one_of`), so the
/// description simply rides on the outer wrapper. The "not nullable +
/// description + reference" case differs between OAS versions: under
/// 3.0 we have to wrap because `$ref` siblings are ignored on the wire,
/// but under 3.1 a sibling `description` is valid so the renderer emits
/// it next to the `$ref`.
fn attach_description(schema: ObjectSchema, description: Option<&str>) -> ObjectSchema {
    let description = match description {
        Some(d) if !d.is_empty() => d.to_string(),
        _ => return schema,
    };
    match (schema.reference.is_some(), is_oas_3_0()) {
        // Bare `$ref` + description under OAS 3.0 — siblings are
        // silently ignored, so we wrap the reference in `allOf` and
        // put the description on the outer schema.
        (true, true) => ObjectSchema {
            description: Some(description),
            all_of: Some(vec![schema]),
            ..ObjectSchema::empty()
        },
        // Bare `$ref` + description under OAS 3.1 — sibling keys are
        // permitted, so we keep the `$ref` schema and attach the
        // description directly. The renderer emits both siblings.
        (true, false) => {
            let mut out = schema;
            out.description = Some(description);
            out
        }
        // Non-`$ref` schema (scalar, array, `allOf` wrap for
        // `Option<Reference>` under 3.0, `oneOf` wrap under 3.1) —
        // description attaches directly.
        (false, _) => {
            let mut out = schema;
            out.description = Some(description);
            out
        }
    }
}

#[cfg(feature = "oas-3-0")]
const fn is_oas_3_0() -> bool {
    true
}

#[cfg(feature = "oas-3-1")]
const fn is_oas_3_0() -> bool {
    false
}

/// Single mapping from a [`PropertyType`] to the matching object schema.
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
fn property_type_to_object_schema(ty: &PropertyType) -> ObjectSchema {
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
            return ObjectSchema {
                ty: Some(SchemaType::Array),
                items: Some(Box::new(property_type_to_object_schema(inner))),
                ..ObjectSchema::empty()
            };
        }
        PropertyType::Nullable(inner) => {
            // A nullable reference cannot simply set `nullable: true` on a
            // `$ref` schema object — `$ref` siblings are ignored in OAS 3.0
            // and disallowed in 3.1. Use the version-specific wrap instead.
            if let PropertyType::Reference(name) = inner.as_ref() {
                return nullable_reference_object(name);
            }
            let mut inner_schema = property_type_to_object_schema(inner);
            inner_schema.nullable = Some(true);
            return inner_schema;
        }
        PropertyType::Reference(name) => {
            return reference_object(name);
        }
    };
    ObjectSchema {
        ty: Some(schema_ty),
        format: format.map(str::to_owned),
        minimum,
        ..ObjectSchema::empty()
    }
}

/// Builds a pure `$ref` object schema pointing at
/// `#/components/schemas/<name>`.
fn reference_object(name: &SchemaName) -> ObjectSchema {
    ObjectSchema {
        reference: Some(format!("#/components/schemas/{}", name.as_str())),
        ..ObjectSchema::empty()
    }
}

/// Builds the OAS-version-specific "nullable reference" object schema.
///
/// - OAS 3.0: `allOf: [{$ref}], nullable: true`. The `allOf` wrap is the
///   idiomatic 3.0 escape hatch for siblings on a referencing schema.
/// - OAS 3.1: `oneOf: [{$ref}, {type: "null"}]`. The `nullable` keyword
///   was dropped in 3.1, and a sibling `$ref` would be rejected.
///
/// The `$ref` element is always first; the null sibling is second. This
/// is purely for snapshot stability.
#[cfg(feature = "oas-3-0")]
fn nullable_reference_object(name: &SchemaName) -> ObjectSchema {
    ObjectSchema {
        all_of: Some(vec![reference_object(name)]),
        nullable: Some(true),
        ..ObjectSchema::empty()
    }
}

#[cfg(feature = "oas-3-1")]
fn nullable_reference_object(name: &SchemaName) -> ObjectSchema {
    ObjectSchema {
        one_of: Some(vec![
            reference_object(name),
            ObjectSchema {
                // `type: "null"` is emitted as a quoted string by the
                // renderer to preserve the user-visible scalar form. See
                // `schema_type_to_value` / `insert_type`.
                ty: Some(SchemaType::Null),
                ..ObjectSchema::empty()
            },
        ]),
        ..ObjectSchema::empty()
    }
}

/// Serializes a [`SchemaObject`] into a [`Value`] by dispatching on the
/// sum and delegating to the variant-specific emitter.
fn schema_object_to_value(schema: &SchemaObject) -> Value {
    match schema {
        SchemaObject::Object(object) => object_schema_to_value(object),
        SchemaObject::StringEnum(string_enum) => string_enum_to_value(string_enum),
        SchemaObject::OneOf(one_of) => one_of_to_value(one_of),
    }
}

/// Serializes a [`OneOfSchema`] into a [`Value`] in the canonical key
/// order `description, oneOf, discriminator` (with `description` emitted
/// only when present).
///
/// Each `oneOf` arm is an `allOf` of two siblings:
///
/// 1. a `$ref` to the variant's inner struct schema,
/// 2. a synthetic object schema constraining the discriminator
///    property to the variant's wire name (`required: [<tag>]`,
///    `properties.<tag>: {type: string, enum: [<wire_name>]}`).
///
/// The synthesized arm 2 is what makes tag dispatch strict on the wire:
/// without it, a reader could accept the wrong tag value and still
/// pass-validate against the inner schema alone.
///
/// The `discriminator` block carries only `propertyName`. The optional
/// `mapping` block is deliberately omitted — a mapping that pointed at
/// `inner_reference` (e.g. `LoginData`) would lead a reader to validate
/// the payload against the inner schema alone, bypassing the
/// `enum: [<wire_name>]` constraint frieze synthesises in the `allOf`.
/// Omitting `mapping` makes readers shape-match across the arms and
/// keeps the tag-value constraint strict.
fn one_of_to_value(schema: &OneOfSchema) -> Value {
    let mut map = Mapping::new();
    if let Some(description) = &schema.description {
        map.insert(
            Value::String("description".into()),
            Value::String(description.clone()),
        );
    }
    let arms: Vec<Value> = schema
        .variants
        .iter()
        .map(|variant| one_of_arm_to_value(&schema.tag, variant))
        .collect();
    map.insert(Value::String("oneOf".into()), Value::Sequence(arms));
    let mut discriminator = Mapping::new();
    discriminator.insert(
        Value::String("propertyName".into()),
        Value::String(schema.tag.clone()),
    );
    map.insert(
        Value::String("discriminator".into()),
        Value::Mapping(discriminator),
    );
    Value::Mapping(map)
}

/// Builds one `oneOf` arm — an `allOf` of `[{$ref}, {synthetic tag object}]`.
fn one_of_arm_to_value(tag: &str, variant: &OneOfVariant) -> Value {
    let mut inner_ref = Mapping::new();
    inner_ref.insert(
        Value::String("$ref".into()),
        Value::String(variant.inner_reference.clone()),
    );

    let mut tag_property = Mapping::new();
    tag_property.insert(
        Value::String("type".into()),
        schema_type_to_value(SchemaType::String),
    );
    tag_property.insert(
        Value::String("enum".into()),
        Value::Sequence(vec![Value::String(variant.wire_name.clone())]),
    );

    let mut tag_properties = Mapping::new();
    tag_properties.insert(Value::String(tag.into()), Value::Mapping(tag_property));

    let mut tag_object = Mapping::new();
    tag_object.insert(
        Value::String("type".into()),
        schema_type_to_value(SchemaType::Object),
    );
    tag_object.insert(
        Value::String("required".into()),
        Value::Sequence(vec![Value::String(tag.into())]),
    );
    tag_object.insert(
        Value::String("properties".into()),
        Value::Mapping(tag_properties),
    );

    let mut arm = Mapping::new();
    arm.insert(
        Value::String("allOf".into()),
        Value::Sequence(vec![Value::Mapping(inner_ref), Value::Mapping(tag_object)]),
    );
    Value::Mapping(arm)
}

/// Serializes a [`StringEnumSchema`] in the canonical key order
/// (`type, description, enum`). `description` is emitted only when
/// present. Variant values are emitted in source order, not
/// alphabetical — matching what serde produces on the wire and what
/// the user reads in the Rust source.
fn string_enum_to_value(schema: &StringEnumSchema) -> Value {
    let mut map = Mapping::new();
    map.insert(
        Value::String("type".into()),
        schema_type_to_value(SchemaType::String),
    );
    if let Some(description) = &schema.description {
        map.insert(
            Value::String("description".into()),
            Value::String(description.clone()),
        );
    }
    let values: Vec<Value> = schema
        .values
        .iter()
        .map(|v| Value::String(v.clone()))
        .collect();
    map.insert(Value::String("enum".into()), Value::Sequence(values));
    Value::Mapping(map)
}

/// Serializes an [`ObjectSchema`] into a [`Value`] with manually ordered
/// keys so that the YAML output is stable.
///
/// Canonical key order (single global rule; each schema kind emits the
/// subset of keys that apply):
///
/// ```text
/// $ref, type, description, format, minimum, items, required,
/// properties, allOf, oneOf, nullable, enum
/// ```
///
/// Per-version differences:
///
/// - `oas-3-0`: a nullable scalar emits a `nullable: true` key.
///   A nullable reference is wrapped in `allOf` with `nullable: true`
///   on the wrapper.
/// - `oas-3-1`: a nullable scalar folds the `null` intent into `type`
///   as a 2-element sequence `[<base>, "null"]`; the `nullable` key is
///   never emitted. A nullable reference is rendered as `oneOf` with a
///   `{type: "null"}` sibling.
///
/// `items` is emitted on array schemas only; the element schema is
/// rendered through the same `object_schema_to_value` recursively so its
/// own keys obey the same ordering rules (and so a nullable item gets
/// its `nullable` marker at the items level, not the array level).
///
/// When `$ref` is present:
///
/// - under `oas-3-0`, no other keys are emitted — the spec says siblings
///   of `$ref` are ignored on the wire, so we strip them at emission to
///   keep the output unambiguous.
/// - under `oas-3-1`, a sibling `description` is allowed and is emitted
///   next to the `$ref`. Other sibling fields are still dropped — the
///   conversion layer is responsible for producing a wrapper schema if
///   they need to be expressed.
fn object_schema_to_value(schema: &ObjectSchema) -> Value {
    let mut map = Mapping::new();
    if let Some(reference) = &schema.reference {
        map.insert(
            Value::String("$ref".into()),
            Value::String(reference.clone()),
        );
        insert_reference_siblings(&mut map, schema);
        return Value::Mapping(map);
    }
    insert_type(&mut map, schema);
    if let Some(description) = &schema.description {
        map.insert(
            Value::String("description".into()),
            Value::String(description.clone()),
        );
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
    if let Some(items) = &schema.items {
        map.insert(Value::String("items".into()), object_schema_to_value(items));
    }
    if !schema.required.is_empty() {
        let items: Vec<Value> = schema
            .required
            .iter()
            .map(|name| Value::String(name.clone()))
            .collect();
        map.insert(Value::String("required".into()), Value::Sequence(items));
    }
    if let Some(properties) = &schema.properties {
        let mut inner = Mapping::new();
        for (name, child) in properties {
            inner.insert(Value::String(name.clone()), object_schema_to_value(child));
        }
        map.insert(Value::String("properties".into()), Value::Mapping(inner));
    }
    if let Some(all_of) = &schema.all_of {
        map.insert(
            Value::String("allOf".into()),
            Value::Sequence(all_of.iter().map(object_schema_to_value).collect()),
        );
    }
    if let Some(one_of) = &schema.one_of {
        map.insert(
            Value::String("oneOf".into()),
            Value::Sequence(one_of.iter().map(object_schema_to_value).collect()),
        );
    }
    insert_nullable(&mut map, schema);
    Value::Mapping(map)
}

/// Emits the sibling keys allowed next to `$ref` on the active OAS
/// version. Under 3.0 nothing is emitted (siblings are spec-ignored);
/// under 3.1, `description` is allowed as a sibling and is emitted in
/// the canonical post-`$ref` position.
#[cfg(feature = "oas-3-0")]
fn insert_reference_siblings(_map: &mut Mapping, _schema: &ObjectSchema) {
    // OAS 3.0: `$ref` siblings are ignored on the wire. Any sibling
    // intent (e.g. a `description` next to a reference) must have been
    // re-shaped into an `allOf` wrap upstream; nothing to emit here.
}

#[cfg(feature = "oas-3-1")]
fn insert_reference_siblings(map: &mut Mapping, schema: &ObjectSchema) {
    if let Some(description) = &schema.description {
        map.insert(
            Value::String("description".into()),
            Value::String(description.clone()),
        );
    }
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
fn insert_type(map: &mut Mapping, schema: &ObjectSchema) {
    if let Some(ty) = schema.ty {
        map.insert(Value::String("type".into()), schema_type_to_value(ty));
    }
}

#[cfg(feature = "oas-3-1")]
fn insert_type(map: &mut Mapping, schema: &ObjectSchema) {
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
fn insert_nullable(map: &mut Mapping, schema: &ObjectSchema) {
    if schema.nullable == Some(true) {
        map.insert(Value::String("nullable".into()), Value::Bool(true));
    }
}

#[cfg(feature = "oas-3-1")]
fn insert_nullable(_map: &mut Mapping, _schema: &ObjectSchema) {
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
/// `ObjectSchema.minimum` is typed as `f64` so the API can carry fractional
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
            frieze_model::Schema::new_object(
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
        assert_eq!(keys, vec!["type", "required", "properties"]);

        let properties = user
            .get(Value::String("properties".into()))
            .and_then(Value::as_mapping)
            .unwrap();
        let property_keys: Vec<&str> = properties.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(property_keys, vec!["id", "name"]);
    }
}
