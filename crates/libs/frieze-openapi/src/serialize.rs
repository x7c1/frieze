//! Runtime dispatch of OAS-version-specific serialization.
//!
//! The schema types in this crate serialize in two distinct forms:
//!
//! - **Canonical (version-neutral)**: the derived `Serialize` impl on
//!   each type. The output mirrors the struct fields one-to-one
//!   (`$ref`, `description` and `nullable` appear as plain sibling
//!   keys, even on a reference) and round-trips through the derived
//!   `Deserialize`. This is the form for machine-readable dumps of
//!   [`Components`] exchanged between tools.
//! - **Versioned (OAS wire form)**: produced by the crate-private
//!   [`Versioned`] wrapper defined here. This is where OAS 3.0 and
//!   3.1 genuinely differ — nullability encoding and `$ref`-sibling
//!   handling — so the wrapper carries a [`Version`] alongside the
//!   value and selects the emitter with a runtime `match`.
//!
//! [`Document`]'s own `Serialize` impl wraps itself in [`Versioned`]
//! with its `oas_version` field, so `serde_yaml::to_string(&doc)` /
//! [`crate::to_yaml`] transparently emit the version the document
//! declares. A single build can therefore serialize 3.0 and 3.1
//! documents side by side; the version is per-document data, not a
//! compile-time choice.
//!
//! # Per-version encoding rules
//!
//! Within an object schema the canonical OAS key order is
//! `$ref, type, description, format, minimum, items, required,
//! properties, allOf, oneOf, nullable`. On top of that order:
//!
//! - **OAS 3.0**: nullability is emitted as `nullable: true` at the
//!   tail position and `type` stays a scalar string. Because `$ref`
//!   siblings are ignored on the 3.0 wire, a reference carrying a
//!   `description` and/or `nullable` intent is wrapped —
//!   `{description?, allOf: [{$ref}], nullable?}` — so the sibling
//!   keys sit on the outer schema.
//! - **OAS 3.1**: nullability folds into `type` as the 2-element
//!   sequence `[<base>, "null"]`; no `nullable` key is emitted. A
//!   sibling `description` is valid next to `$ref` and is emitted in
//!   the post-`$ref` position. A nullable reference becomes
//!   `{description?, oneOf: [{$ref}, {type: "null"}]}` since 3.1
//!   dropped the `nullable` keyword.

use indexmap::IndexMap;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

use crate::components::Components;
use crate::document::Document;
use crate::object_schema::ObjectSchema;
use crate::one_of_schema::{OneOfSchema, OneOfVariant};
use crate::schema_object::SchemaObject;
use crate::schema_type::SchemaType;
use crate::string_enum_schema::StringEnumSchema;
use crate::version::Version;

/// Crate-private wrapper that pairs a value with the [`Version`] it
/// should be emitted as.
///
/// The wrapper is the propagation vehicle: each `Serialize` impl below
/// re-wraps its children so the version travels down the whole
/// document tree without living inside the data types themselves.
pub(crate) struct Versioned<'a, T> {
    value: &'a T,
    version: Version,
}

impl<'a, T> Versioned<'a, T> {
    pub(crate) fn new(value: &'a T, version: Version) -> Self {
        Self { value, version }
    }
}

impl Serialize for Versioned<'_, Document> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let document = self.value;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("openapi", &document.openapi)?;
        map.serialize_entry("info", &document.info)?;
        if let Some(servers) = &document.servers {
            map.serialize_entry("servers", servers)?;
        }
        if let Some(paths) = &document.paths {
            map.serialize_entry("paths", paths)?;
        }
        if let Some(components) = &document.components {
            map.serialize_entry("components", &Versioned::new(components, self.version))?;
        }
        if let Some(security) = &document.security {
            map.serialize_entry("security", security)?;
        }
        if let Some(tags) = &document.tags {
            map.serialize_entry("tags", tags)?;
        }
        if let Some(external_docs) = &document.external_docs {
            map.serialize_entry("externalDocs", external_docs)?;
        }
        for (key, value) in &document.extensions {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl Serialize for Versioned<'_, Components> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let components = self.value;
        let mut map = serializer.serialize_map(None)?;
        if !components.schemas.is_empty() {
            map.serialize_entry(
                "schemas",
                &Versioned::new(&components.schemas, self.version),
            )?;
        }
        for (key, value) in &components.other {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// Version propagation through the `components.schemas` map.
///
/// The map / list impls below are written per concrete element type
/// (rather than as blanket impls over any `Versioned`-serializable
/// `T`) because a blanket `Versioned<Container<T>>: Serialize where
/// Versioned<T>: Serialize` impl is self-recursive from the trait
/// solver's point of view and overflows the recursion limit.
impl Serialize for Versioned<'_, IndexMap<String, SchemaObject>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.value.len()))?;
        for (key, value) in self.value {
            map.serialize_entry(key, &Versioned::new(value, self.version))?;
        }
        map.end()
    }
}

/// Version propagation through a `properties` map.
impl Serialize for Versioned<'_, IndexMap<String, ObjectSchema>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.value.len()))?;
        for (key, value) in self.value {
            map.serialize_entry(key, &Versioned::new(value, self.version))?;
        }
        map.end()
    }
}

/// Version propagation through a schema list (`allOf`, `oneOf`).
impl Serialize for Versioned<'_, Vec<ObjectSchema>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.value.len()))?;
        for item in self.value {
            seq.serialize_element(&Versioned::new(item, self.version))?;
        }
        seq.end()
    }
}

/// Dispatch-only impl: delegates straight to the active variant's
/// versioned serializer so each variant controls its own canonical OAS
/// key order.
impl Serialize for Versioned<'_, SchemaObject> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.value {
            SchemaObject::OneOf(one_of) => {
                Versioned::new(one_of, self.version).serialize(serializer)
            }
            SchemaObject::StringEnum(string_enum) => {
                Versioned::new(string_enum, self.version).serialize(serializer)
            }
            SchemaObject::Object(object) => {
                Versioned::new(object, self.version).serialize(serializer)
            }
        }
    }
}

/// OAS emission for [`ObjectSchema`] — the one place where the OAS
/// 3.0 / 3.1 wire forms actually diverge. See the module docs for the
/// per-version rules.
impl Serialize for Versioned<'_, ObjectSchema> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let schema = self.value;
        let mut map = serializer.serialize_map(None)?;

        if let Some(reference) = &schema.reference {
            match self.version {
                Version::V3_0 => serialize_reference_v3_0(&mut map, reference, schema)?,
                Version::V3_1 => serialize_reference_v3_1(&mut map, reference, schema)?,
            }
            return map.end();
        }

        if let Some(ty) = schema.ty {
            match self.version {
                // Under 3.0 `type` is always a scalar string; the
                // nullability marker is emitted separately below.
                Version::V3_0 => map.serialize_entry("type", &ty)?,
                // Under 3.1 the nullability marker folds into `type`
                // as the 2-element sequence `[<base>, "null"]`.
                Version::V3_1 => {
                    if schema.nullable == Some(true) {
                        map.serialize_entry("type", &TypeWithNull(ty))?;
                    } else {
                        map.serialize_entry("type", &ty)?;
                    }
                }
            }
        }
        if let Some(description) = &schema.description {
            map.serialize_entry("description", description)?;
        }
        if let Some(format) = &schema.format {
            map.serialize_entry("format", format)?;
        }
        if let Some(minimum) = schema.minimum {
            map.serialize_entry("minimum", &Minimum(minimum))?;
        }
        if let Some(items) = &schema.items {
            map.serialize_entry("items", &Versioned::new(items.as_ref(), self.version))?;
        }
        if !schema.required.is_empty() {
            map.serialize_entry("required", &schema.required)?;
        }
        if let Some(properties) = &schema.properties {
            map.serialize_entry("properties", &Versioned::new(properties, self.version))?;
        }
        if let Some(all_of) = &schema.all_of {
            map.serialize_entry("allOf", &Versioned::new(all_of, self.version))?;
        }
        if let Some(one_of) = &schema.one_of {
            map.serialize_entry("oneOf", &Versioned::new(one_of, self.version))?;
        }
        if self.version == Version::V3_0 && schema.nullable == Some(true) {
            map.serialize_entry("nullable", &true)?;
        }

        map.end()
    }
}

/// Emits a reference schema on the OAS 3.0 wire.
///
/// `$ref` siblings are ignored on the 3.0 wire, so a reference that
/// carries a `description` and/or `nullable` intent cannot emit them
/// as siblings. Instead the reference is wrapped:
/// `{description?, allOf: [{$ref}], nullable?}` — the sibling keys sit
/// on the outer schema and the `$ref` moves into a single-element
/// `allOf`. A bare reference (no description, not nullable) emits
/// `$ref` alone.
fn serialize_reference_v3_0<M>(
    map: &mut M,
    reference: &str,
    schema: &ObjectSchema,
) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    let nullable = schema.nullable == Some(true);
    if !nullable && schema.description.is_none() {
        map.serialize_entry("$ref", reference)?;
        return Ok(());
    }
    if let Some(description) = &schema.description {
        map.serialize_entry("description", description)?;
    }
    map.serialize_entry("allOf", &SingleRefList { reference })?;
    if nullable {
        map.serialize_entry("nullable", &true)?;
    }
    Ok(())
}

/// Emits a reference schema on the OAS 3.1 wire.
///
/// A sibling `description` is valid next to `$ref` under 3.1 and is
/// emitted in the canonical post-`$ref` position. A nullable reference
/// becomes `{description?, oneOf: [{$ref}, {type: "null"}]}` — 3.1
/// dropped the `nullable` keyword, so null-ness is expressed as a
/// `oneOf` against the `"null"` type.
fn serialize_reference_v3_1<M>(
    map: &mut M,
    reference: &str,
    schema: &ObjectSchema,
) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    if schema.nullable == Some(true) {
        if let Some(description) = &schema.description {
            map.serialize_entry("description", description)?;
        }
        map.serialize_entry("oneOf", &NullableRefArms { reference })?;
        return Ok(());
    }
    map.serialize_entry("$ref", reference)?;
    if let Some(description) = &schema.description {
        map.serialize_entry("description", description)?;
    }
    Ok(())
}

/// `[{$ref: <reference>}]` — the single-element `allOf` list used by
/// the OAS 3.0 wraps.
struct SingleRefList<'a> {
    reference: &'a str,
}

impl Serialize for SingleRefList<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(1))?;
        seq.serialize_element(&RefObject {
            reference: self.reference,
        })?;
        seq.end()
    }
}

/// `[{$ref: <reference>}, {type: "null"}]` — the two `oneOf` arms of
/// the OAS 3.1 nullable-reference shape. The `$ref` element is always
/// first; the null arm is second (for snapshot stability).
struct NullableRefArms<'a> {
    reference: &'a str,
}

impl Serialize for NullableRefArms<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&RefObject {
            reference: self.reference,
        })?;
        seq.serialize_element(&NullTypeObject)?;
        seq.end()
    }
}

/// `{$ref: <reference>}` — a schema object holding nothing but a
/// reference.
struct RefObject<'a> {
    reference: &'a str,
}

impl Serialize for RefObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("$ref", self.reference)?;
        map.end()
    }
}

/// `{type: "null"}` — the null arm of the OAS 3.1 nullable-reference
/// shape. [`SchemaType::Null`] serializes to the quoted string
/// `"null"`, not the YAML null value.
struct NullTypeObject;

impl Serialize for NullTypeObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("type", &SchemaType::Null)?;
        map.end()
    }
}

/// `Serialize` wrapper for the `minimum` field that emits a
/// whole-number bound as an integer (the OAS-idiomatic `minimum: 0`
/// rather than `0.0`), and any fractional bound as a float.
///
/// `ObjectSchema.minimum` is typed as `f64` to leave room for
/// fractional bounds, but the only values the derive currently
/// produces are integer constants (`0` for `u32` / `u64`), which
/// should render as integers on the wire.
struct Minimum(f64);

impl Serialize for Minimum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self.0;
        let as_int = value as i64;
        if value.is_finite() && (as_int as f64) == value {
            serializer.serialize_i64(as_int)
        } else {
            serializer.serialize_f64(value)
        }
    }
}

/// `Serialize` wrapper that emits a `type` field as the 2-element
/// sequence `[<base>, "null"]` used by OAS 3.1 to express nullability.
struct TypeWithNull(SchemaType);

impl Serialize for TypeWithNull {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.0)?;
        seq.serialize_element(&SchemaType::Null)?;
        seq.end()
    }
}

/// OAS emission for [`StringEnumSchema`]: canonical key order
/// `type, description, enum` (with `description` emitted only when
/// present). The shape is identical under OAS 3.0 and 3.1 — no
/// nullability or `$ref` siblings are involved — so no version match
/// is needed; the impl lives on the wrapper so the OAS wire form and
/// the canonical (derived) form stay two separate paths.
impl Serialize for Versioned<'_, StringEnumSchema> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let schema = self.value;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", &SchemaType::String)?;
        if let Some(description) = &schema.description {
            map.serialize_entry("description", description)?;
        }
        map.serialize_entry("enum", &schema.values)?;
        map.end()
    }
}

/// OAS emission for [`OneOfSchema`]: canonical key order
/// `description, oneOf, discriminator` (with `description` emitted
/// only when present). Identical under OAS 3.0 and 3.1.
///
/// Each `oneOf` arm renders as
/// `{allOf: [{$ref: <inner_reference>}, {type: object, required: [<tag>],
/// properties: {<tag>: {type: string, enum: [<wire_name>]}}}]}` so that
/// readers must shape-match both the inner struct schema and the
/// discriminator-property constraint. The enclosing schema's
/// `discriminator` block only carries `propertyName` — the optional
/// `mapping` block is deliberately omitted (a `mapping` that pointed at
/// `inner_reference` would let a reader bypass the `enum: [<wire_name>]`
/// constraint by validating the payload against the inner schema alone).
impl Serialize for Versioned<'_, OneOfSchema> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let schema = self.value;
        let mut map = serializer.serialize_map(None)?;
        if let Some(description) = &schema.description {
            map.serialize_entry("description", description)?;
        }
        map.serialize_entry("oneOf", &OneOfArms { schema })?;
        map.serialize_entry("discriminator", &Discriminator { tag: &schema.tag })?;
        map.end()
    }
}

/// Adapter that serializes the variant list as a sequence of
/// synthesized `allOf` arms.
struct OneOfArms<'a> {
    schema: &'a OneOfSchema,
}

impl Serialize for OneOfArms<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.schema.variants.len()))?;
        for variant in &self.schema.variants {
            seq.serialize_element(&OneOfArm {
                tag: &self.schema.tag,
                variant,
            })?;
        }
        seq.end()
    }
}

/// One `allOf` arm — `[{$ref}, {synthetic tag-property object}]`.
struct OneOfArm<'a> {
    tag: &'a str,
    variant: &'a OneOfVariant,
}

impl Serialize for OneOfArm<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut arm = serializer.serialize_map(Some(1))?;
        arm.serialize_entry(
            "allOf",
            &AllOfPair {
                tag: self.tag,
                variant: self.variant,
            },
        )?;
        arm.end()
    }
}

struct AllOfPair<'a> {
    tag: &'a str,
    variant: &'a OneOfVariant,
}

impl Serialize for AllOfPair<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&RefObject {
            reference: &self.variant.inner_reference,
        })?;
        seq.serialize_element(&TagPropertyObject {
            tag: self.tag,
            wire_name: &self.variant.wire_name,
        })?;
        seq.end()
    }
}

/// Renders the synthetic discriminator arm:
/// `{type: object, required: [<tag>], properties: {<tag>: {type: string,
/// enum: [<wire_name>]}}}`.
struct TagPropertyObject<'a> {
    tag: &'a str,
    wire_name: &'a str,
}

impl Serialize for TagPropertyObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("type", &SchemaType::Object)?;
        map.serialize_entry("required", &[self.tag])?;
        map.serialize_entry(
            "properties",
            &TagProperties {
                tag: self.tag,
                wire_name: self.wire_name,
            },
        )?;
        map.end()
    }
}

struct TagProperties<'a> {
    tag: &'a str,
    wire_name: &'a str,
}

impl Serialize for TagProperties<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(
            self.tag,
            &TagPropertyConstraint {
                wire_name: self.wire_name,
            },
        )?;
        map.end()
    }
}

struct TagPropertyConstraint<'a> {
    wire_name: &'a str,
}

impl Serialize for TagPropertyConstraint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", &SchemaType::String)?;
        map.serialize_entry("enum", &[self.wire_name])?;
        map.end()
    }
}

struct Discriminator<'a> {
    tag: &'a str,
}

impl Serialize for Discriminator<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("propertyName", self.tag)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::Info;
    use std::collections::BTreeMap;

    /// Renders a value through the versioned OAS emitter.
    fn render<T>(value: &T, version: Version) -> String
    where
        for<'a> Versioned<'a, T>: Serialize,
    {
        serde_yaml::to_string(&Versioned::new(value, version))
            .expect("YAML serialization must succeed")
    }

    #[test]
    fn object_schema_emits_canonical_key_order_under_both_versions() {
        let mut properties: IndexMap<String, ObjectSchema> = IndexMap::new();
        properties.insert(
            "id".to_string(),
            ObjectSchema {
                ty: Some(SchemaType::Integer),
                format: Some("int64".to_string()),
                ..ObjectSchema::empty()
            },
        );
        properties.insert(
            "name".to_string(),
            ObjectSchema {
                ty: Some(SchemaType::String),
                ..ObjectSchema::empty()
            },
        );
        let schema = ObjectSchema {
            ty: Some(SchemaType::Object),
            required: vec!["id".to_string(), "name".to_string()],
            properties: Some(properties),
            ..ObjectSchema::empty()
        };

        let expected = "\
type: object
required:
- id
- name
properties:
  id:
    type: integer
    format: int64
  name:
    type: string
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn minimum_zero_emits_integer_not_float() {
        // `minimum: 0` (not `0.0`) is the OAS-idiomatic shape for an
        // unsigned scalar bound. The versioned emitter falls back to a
        // float only when the bound carries fractional information.
        let schema = ObjectSchema {
            ty: Some(SchemaType::Integer),
            format: Some("int32".to_string()),
            minimum: Some(0.0),
            ..ObjectSchema::empty()
        };
        let expected = "\
type: integer
format: int32
minimum: 0
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn array_items_recurse_through_the_versioned_emitter() {
        let schema = ObjectSchema {
            ty: Some(SchemaType::Array),
            items: Some(Box::new(ObjectSchema {
                ty: Some(SchemaType::String),
                ..ObjectSchema::empty()
            })),
            ..ObjectSchema::empty()
        };
        let expected = "\
type: array
items:
  type: string
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn bare_reference_emits_ref_alone_under_both_versions() {
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/User".to_string()),
            ..ObjectSchema::empty()
        };
        let expected = "\
$ref: '#/components/schemas/User'
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn reference_with_description_wraps_in_all_of_under_3_0() {
        // `$ref` siblings are ignored on the OAS 3.0 wire, so the
        // description moves to an outer schema that wraps the
        // reference in a single-element `allOf`.
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/User".to_string()),
            description: Some("The current user.".to_string()),
            ..ObjectSchema::empty()
        };
        let expected = "\
description: The current user.
allOf:
- $ref: '#/components/schemas/User'
";
        assert_eq!(render(&schema, Version::V3_0), expected);
    }

    #[test]
    fn reference_with_description_emits_sibling_under_3_1() {
        // OAS 3.1 allows sibling `description` next to `$ref`; it is
        // emitted in the canonical post-`$ref` position.
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/User".to_string()),
            description: Some("The current user.".to_string()),
            ..ObjectSchema::empty()
        };
        let expected = "\
$ref: '#/components/schemas/User'
description: The current user.
";
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn nullable_reference_wraps_in_all_of_with_nullable_under_3_0() {
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/Inner".to_string()),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected = "\
allOf:
- $ref: '#/components/schemas/Inner'
nullable: true
";
        assert_eq!(render(&schema, Version::V3_0), expected);
    }

    #[test]
    fn nullable_reference_becomes_one_of_with_null_under_3_1() {
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/Inner".to_string()),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected = "\
oneOf:
- $ref: '#/components/schemas/Inner'
- type: 'null'
";
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn nullable_reference_with_description_keeps_description_on_the_wrapper() {
        let schema = ObjectSchema {
            reference: Some("#/components/schemas/Inner".to_string()),
            description: Some("May be absent.".to_string()),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected_3_0 = "\
description: May be absent.
allOf:
- $ref: '#/components/schemas/Inner'
nullable: true
";
        let expected_3_1 = "\
description: May be absent.
oneOf:
- $ref: '#/components/schemas/Inner'
- type: 'null'
";
        assert_eq!(render(&schema, Version::V3_0), expected_3_0);
        assert_eq!(render(&schema, Version::V3_1), expected_3_1);
    }

    #[test]
    fn nullable_scalar_emits_nullable_true_under_3_0() {
        let schema = ObjectSchema {
            ty: Some(SchemaType::String),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected = "\
type: string
nullable: true
";
        assert_eq!(render(&schema, Version::V3_0), expected);
    }

    #[test]
    fn nullable_scalar_emits_type_sequence_under_3_1() {
        let schema = ObjectSchema {
            ty: Some(SchemaType::String),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected = "\
type:
- string
- 'null'
";
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn explicit_all_of_wrap_round_trips_the_3_0_wire_shape() {
        // A document parsed from the 3.0 wire carries the wrap as
        // explicit data (`all_of` + `nullable`); re-emitting it under
        // 3.0 reproduces the same bytes.
        let schema = ObjectSchema {
            all_of: Some(vec![ObjectSchema {
                reference: Some("#/components/schemas/Inner".to_string()),
                ..ObjectSchema::empty()
            }]),
            nullable: Some(true),
            ..ObjectSchema::empty()
        };
        let expected = "\
allOf:
- $ref: '#/components/schemas/Inner'
nullable: true
";
        assert_eq!(render(&schema, Version::V3_0), expected);
    }

    #[test]
    fn explicit_one_of_wrap_round_trips_the_3_1_wire_shape() {
        let schema = ObjectSchema {
            one_of: Some(vec![
                ObjectSchema {
                    reference: Some("#/components/schemas/Inner".to_string()),
                    ..ObjectSchema::empty()
                },
                ObjectSchema {
                    ty: Some(SchemaType::Null),
                    ..ObjectSchema::empty()
                },
            ]),
            ..ObjectSchema::empty()
        };
        let expected = "\
oneOf:
- $ref: '#/components/schemas/Inner'
- type: 'null'
";
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn one_of_schema_emits_synthetic_discriminator_arm_and_omits_mapping() {
        let schema = OneOfSchema::new(
            "kind",
            vec![
                OneOfVariant {
                    wire_name: "Login".to_string(),
                    inner_reference: "#/components/schemas/LoginData".to_string(),
                },
                OneOfVariant {
                    wire_name: "Logout".to_string(),
                    inner_reference: "#/components/schemas/LogoutData".to_string(),
                },
            ],
        );
        let expected = "\
oneOf:
- allOf:
  - $ref: '#/components/schemas/LoginData'
  - type: object
    required:
    - kind
    properties:
      kind:
        type: string
        enum:
        - Login
- allOf:
  - $ref: '#/components/schemas/LogoutData'
  - type: object
    required:
    - kind
    properties:
      kind:
        type: string
        enum:
        - Logout
discriminator:
  propertyName: kind
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn one_of_schema_with_description_emits_description_first() {
        let schema = OneOfSchema::new(
            "kind",
            vec![OneOfVariant {
                wire_name: "Only".to_string(),
                inner_reference: "#/components/schemas/Only".to_string(),
            }],
        )
        .with_description(Some("An internally-tagged enum.".to_string()));
        let rendered = render(&schema, Version::V3_0);
        let first_line = rendered.lines().next().unwrap_or("");
        assert_eq!(first_line, "description: An internally-tagged enum.");
        // The `discriminator` block must carry only `propertyName` — no
        // `mapping` sub-key.
        assert!(
            !rendered.contains("mapping"),
            "discriminator must not emit `mapping`, but the rendered output was:\n{rendered}"
        );
    }

    #[test]
    fn string_enum_schema_emits_type_description_enum_in_canonical_order() {
        let schema = StringEnumSchema::new(vec!["Red".to_string(), "Green".to_string()])
            .with_description(Some("A traffic-light hue.".to_string()));
        let expected = "\
type: string
description: A traffic-light hue.
enum:
- Red
- Green
";
        assert_eq!(render(&schema, Version::V3_0), expected);
        assert_eq!(render(&schema, Version::V3_1), expected);
    }

    #[test]
    fn schema_object_dispatch_matches_each_variant_serialized_directly() {
        let object = SchemaObject::Object(ObjectSchema {
            ty: Some(SchemaType::String),
            ..ObjectSchema::empty()
        });
        let string_enum = SchemaObject::StringEnum(StringEnumSchema::new(vec!["A".to_string()]));
        let one_of = SchemaObject::OneOf(OneOfSchema::new(
            "kind",
            vec![OneOfVariant {
                wire_name: "X".to_string(),
                inner_reference: "#/components/schemas/X".to_string(),
            }],
        ));

        let SchemaObject::Object(ref inner_object) = object else {
            unreachable!()
        };
        assert_eq!(
            render(&object, Version::V3_0),
            render(inner_object, Version::V3_0)
        );

        let SchemaObject::StringEnum(ref inner_string_enum) = string_enum else {
            unreachable!()
        };
        assert_eq!(
            render(&string_enum, Version::V3_0),
            render(inner_string_enum, Version::V3_0)
        );

        let SchemaObject::OneOf(ref inner_one_of) = one_of else {
            unreachable!()
        };
        assert_eq!(
            render(&one_of, Version::V3_0),
            render(inner_one_of, Version::V3_0)
        );
    }

    #[test]
    fn documents_of_both_versions_serialize_side_by_side_in_one_build() {
        // The whole point of runtime dispatch: two documents that
        // differ only in their version emit their respective wire
        // shapes from the same build, at the same time.
        let document_for = |version: Version| {
            let mut schemas: IndexMap<String, SchemaObject> = IndexMap::new();
            schemas.insert(
                "Wrapper".to_string(),
                SchemaObject::Object(ObjectSchema {
                    reference: Some("#/components/schemas/Inner".to_string()),
                    nullable: Some(true),
                    ..ObjectSchema::empty()
                }),
            );
            Document::from_components(
                Info {
                    title: "t".to_string(),
                    version: "v".to_string(),
                    description: None,
                    extensions: BTreeMap::new(),
                },
                Components {
                    schemas,
                    other: BTreeMap::new(),
                },
                version,
            )
        };

        let yaml_3_0 = serde_yaml::to_string(&document_for(Version::V3_0)).unwrap();
        let yaml_3_1 = serde_yaml::to_string(&document_for(Version::V3_1)).unwrap();

        let expected_3_0 = "\
openapi: 3.0.3
info:
  title: t
  version: v
components:
  schemas:
    Wrapper:
      allOf:
      - $ref: '#/components/schemas/Inner'
      nullable: true
";
        let expected_3_1 = "\
openapi: 3.1.0
info:
  title: t
  version: v
components:
  schemas:
    Wrapper:
      oneOf:
      - $ref: '#/components/schemas/Inner'
      - type: 'null'
";
        assert_eq!(yaml_3_0, expected_3_0);
        assert_eq!(yaml_3_1, expected_3_1);
    }
}
