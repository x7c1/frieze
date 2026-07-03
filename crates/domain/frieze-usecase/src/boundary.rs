//! Boundary conversion from a domain [`frieze_model::Schema`] to the
//! plain OAS [`frieze_openapi::SchemaObject`].
//!
//! This module is the single bridge between the validated domain types
//! produced by `frieze-model` (and assembled by `frieze::SchemasBuilder`) and
//! the OAS-shaped types in `frieze-openapi`. Everything downstream of
//! this layer — `compose`, `from_schemas`, and any output renderer —
//! operates on `frieze-openapi` types only.
//!
//! The conversion is a pure, **version-neutral** data mapping: it
//! records intent (`reference`, `description`, `nullable`) as plain
//! fields on the produced [`ObjectSchema`]s and never applies the OAS
//! 3.0 / 3.1 encoding split. That split (nullable encoding,
//! `$ref`-sibling handling) is applied at serialization time by
//! `frieze-openapi`, dispatching on the document's `oas_version`.
//!
//! The per-schema conversion is intentionally local to the use-case
//! crate; only the aggregate entry point
//! ([`components_from_schemas`]) is re-exported, so the dependency
//! direction stays `frieze-usecase → frieze-model, frieze-openapi`
//! without leaking domain types out through the public API.

use frieze_model::{
    primitive_property_type_for, Property, PropertyType, Schema, SchemaName, Schemas,
};
use frieze_openapi::{
    Components, ObjectSchema, OneOfSchema, OneOfVariant, SchemaObject, SchemaType, StringEnumSchema,
};
use indexmap::IndexMap;

/// Converts a [`Schemas`] collection into the version-neutral,
/// canonical [`Components`] value.
///
/// Each registered schema is mapped through the boundary conversion
/// and inserted under `components.schemas`, keyed by its registration
/// name, in the collection's canonical (alphabetical) order. The
/// result carries no OAS-version-specific encoding: nullability and
/// `$ref`-companion intents are stored as plain fields, and the OAS
/// 3.0 / 3.1 wire shapes are produced later, at serialization time,
/// from the version of whichever `Document` the components end up in.
///
/// Because the canonical form is version-neutral, it is also the
/// interchange format between processes: serializing the returned
/// [`Components`] with its derived `Serialize` (e.g. via
/// `serde_json::to_writer`) produces a dump that another process can
/// parse back with the derived `Deserialize` and compose into
/// documents of any supported OAS version.
pub fn components_from_schemas(schemas: &Schemas) -> Components {
    let mut components = Components::default();
    for (name, schema) in &schemas.by_name {
        if let Some(object) = to_openapi(schema) {
            components.schemas.insert(name.as_str().to_string(), object);
        }
    }
    components
}

/// Boundary conversion: validated domain schema -> plain OAS schema object.
///
/// Each [`Schema`] variant maps to the matching [`SchemaObject`] variant.
/// For [`Schema::Object`], the `required` array is built from each
/// property's [`Property::presence`] — and only that. Value-level
/// nullability lives on the type tree ([`PropertyType::Nullable`]) and is
/// rendered independently by [`property_type_to_object_schema`].
///
/// Returns `None` for [`Schema::Scalar`] — scalar schemas are never
/// emitted under `#/components/schemas`. The primary guard is the
/// `IsRegistrable` marker trait (compile-time); this `None` arm is the
/// defensive secondary guard at the boundary.
fn to_openapi(schema: &Schema) -> Option<SchemaObject> {
    match schema {
        Schema::Object(object) => Some(SchemaObject::Object(object_schema_from_model(object))),
        Schema::StringEnum(string_enum) => Some(SchemaObject::StringEnum(
            StringEnumSchema::new(string_enum.values.clone())
                .with_description(string_enum.description.clone()),
        )),
        Schema::OneOf(one_of) => Some(SchemaObject::OneOf(one_of_from_model(one_of))),
        Schema::Scalar(_) => None,
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
/// by [`property_type_to_object_schema`].
///
/// In the canonical (version-neutral) form the description always
/// attaches directly — even next to a `$ref`, where the OAS 3.0 wire
/// would not allow a sibling key. The version-specific placement (the
/// `allOf` wrap under 3.0, the plain sibling under 3.1) is applied at
/// serialization time by `frieze-openapi`'s versioned emitter.
fn attach_description(mut schema: ObjectSchema, description: Option<&str>) -> ObjectSchema {
    if let Some(description) = description.filter(|d| !d.is_empty()) {
        schema.description = Some(description.to_string());
    }
    schema
}

/// Single mapping from a [`PropertyType`] to the matching object schema.
///
/// Recurses on [`PropertyType::Array`] so the element schema is rendered
/// into the `items` slot, and on [`PropertyType::Nullable`] so the
/// nullability marker is attached at whichever position in the tree it
/// appears. `Array(Nullable(...))` therefore makes the items nullable;
/// `Nullable(Array(...))` makes the array itself nullable.
///
/// [`PropertyType::Reference`] is rendered as a pure `$ref` schema. A
/// `Reference` wrapped in [`PropertyType::Nullable`] renders as that
/// same `$ref` schema with the canonical `nullable` flag set — the
/// version-specific "nullable reference" wire shape (`allOf` +
/// `nullable: true` under OAS 3.0, `oneOf` against `{type: "null"}`
/// under 3.1) is synthesized at serialization time.
///
/// References whose name maps to one of the eight primitive scalar
/// conventions ([`primitive_property_type_for`]) are **inlined** at the
/// leaf position as the matching scalar shape (`{type: integer, format:
/// int64}`, `{type: string}`, ...). Generic derive output for
/// `Container<i64>`-style instantiations cannot tell at macro-expansion
/// time whether the type parameter is a primitive, so it always emits
/// `PropertyType::Reference`; the boundary inlines those references so
/// the OAS document has no dangling `$ref: #/components/schemas/Int64`.
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
            // In the canonical form, nullability is always the plain
            // `nullable` flag — even on a `$ref` schema, where the OAS
            // wire needs a version-specific wrap. Primitive references
            // inline at the leaf first, so the flag lands on the
            // resulting scalar shape.
            if let PropertyType::Reference(name) = inner.as_ref() {
                if let Some(prim) = primitive_property_type_for(name) {
                    let mut inner_schema = property_type_to_object_schema(&prim);
                    inner_schema.nullable = Some(true);
                    return inner_schema;
                }
                let mut reference = reference_object(name);
                reference.nullable = Some(true);
                return reference;
            }
            let mut inner_schema = property_type_to_object_schema(inner);
            inner_schema.nullable = Some(true);
            return inner_schema;
        }
        PropertyType::Reference(name) => {
            if let Some(prim) = primitive_property_type_for(name) {
                return property_type_to_object_schema(&prim);
            }
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
