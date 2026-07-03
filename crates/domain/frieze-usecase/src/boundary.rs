//! Boundary conversion from a domain [`frieze_model::Schema`] to the
//! plain OAS [`frieze_openapi::SchemaObject`].
//!
//! This module is the single bridge between the validated domain types
//! produced by `frieze-model` (and assembled by `frieze::SchemasBuilder`) and
//! the OAS-shaped types in `frieze-openapi`. Everything downstream of
//! this layer — `compose`, `from_schemas`, and any output renderer —
//! operates on `frieze-openapi` types only.
//!
//! The conversion is intentionally local to the use-case crate (and not
//! re-exported) so the dependency direction stays
//! `frieze-usecase → frieze-model, frieze-openapi` without leaking
//! domain types out through the public API.

use frieze_model::{primitive_property_type_for, Property, PropertyType, Schema, SchemaName};
use frieze_openapi::{
    ObjectSchema, OneOfSchema, OneOfVariant, SchemaObject, SchemaType, StringEnumSchema,
};
use indexmap::IndexMap;

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
pub(crate) fn to_openapi(schema: &Schema) -> Option<SchemaObject> {
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
            // A nullable reference cannot simply set `nullable: true` on a
            // `$ref` schema object — `$ref` siblings are ignored in OAS 3.0
            // and disallowed in 3.1. Use the version-specific wrap instead.
            if let PropertyType::Reference(name) = inner.as_ref() {
                // Primitive references inline at the leaf, so the
                // "nullable reference" wrap is unnecessary: emit the
                // scalar shape with the standard scalar nullability
                // treatment (`nullable: true` under 3.0, `type` sequence
                // under 3.1).
                if let Some(prim) = primitive_property_type_for(name) {
                    let mut inner_schema = property_type_to_object_schema(&prim);
                    inner_schema.nullable = Some(true);
                    return inner_schema;
                }
                return nullable_reference_object(name);
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
                // `type: "null"` is emitted by the `SchemaType::Null`
                // variant's `Serialize` impl as the quoted string
                // `"null"` (not the YAML null value).
                ty: Some(SchemaType::Null),
                ..ObjectSchema::empty()
            },
        ]),
        ..ObjectSchema::empty()
    }
}
