//! The "object schema" half of the [`crate::SchemaObject`] sum.
//!
//! This carries every OAS Schema Object key the per-property renderer
//! produces: `$ref`, `type`, `description`, `format`, `minimum`, `items`,
//! `required`, `properties`, `allOf`, `oneOf`, `nullable`. It is used
//! both for top-level object schemas (with `properties` / `required`
//! populated) and for the inner per-property and per-`oneOf`/-`allOf`/
//! -`items` sub-schemas, which leave `properties` empty and populate
//! `ty` / `format` / etc. as needed.

use indexmap::IndexMap;
use serde::ser::SerializeMap;
#[cfg(feature = "oas-3-1")]
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};

use crate::schema_type::SchemaType;

/// The "object schema" variant carried by [`crate::SchemaObject`].
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
///
/// The field order in this struct mirrors the canonical YAML output order
/// (`$ref`, `type`, `description`, `format`, `minimum`, `items`,
/// `required`, `properties`, `allOf`, `oneOf`, `nullable`) so
/// contributors can predict the output shape by reading the struct.
/// Emission itself is performed by the custom emitter in `frieze-usecase`
/// — serde no longer participates.
///
/// A schema object set to a `$ref` is, per OAS, a leaf — when `reference`
/// is set, callers must not also set sibling fields. The renderer in
/// `frieze-usecase` honours this by emitting `$ref` alone when present.
///
/// The `Deserialize` impl here is an intentionally naive auto-derive
/// used only as a transport: it lets `ObjectSchema` values move through
/// the top-level [`crate::OasDocument`] / [`crate::Components`] structs
/// without hand-writing their conversions. The matching `Serialize` impl
/// is handwritten further down to produce the canonical OAS key order
/// and the OAS 3.0 / 3.1 conditional nullability encoding.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ObjectSchema {
    /// JSON pointer to another schema (typically
    /// `#/components/schemas/<name>`). When set, the schema object is a
    /// pure reference: any sibling fields are ignored on the wire.
    #[serde(default, rename = "$ref")]
    pub reference: Option<String>,
    #[serde(default, rename = "type")]
    pub ty: Option<SchemaType>,
    /// Free-form description text. Rendered as the `description` field
    /// of the OAS schema when present. Empty / whitespace-only inputs
    /// are stripped to `None` upstream in `frieze-model` so this side
    /// only ever sees a meaningful value.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    /// Inclusive lower bound for numeric values. Currently used only to
    /// encode Rust's unsigned semantics in OAS (`minimum: 0` for `u32` /
    /// `u64`), since OAS 3.0 has no canonical unsigned representation.
    ///
    /// Stored as `f64` so the type can represent fractional bounds in the
    /// future. Whole-number values are emitted as YAML integers (e.g. `0`
    /// rather than `0.0`); the handwritten `Serialize` impl picks the
    /// integer form whenever the bound round-trips losslessly through
    /// `i64`.
    #[serde(default)]
    pub minimum: Option<f64>,
    /// Element schema for array types. Boxed because [`ObjectSchema`]
    /// references itself recursively here (an array's items are themselves
    /// schema objects).
    #[serde(default)]
    pub items: Option<Box<ObjectSchema>>,
    /// Names of properties that must appear on the wire. Omitted from
    /// the emitted schema entirely when empty (an all-optional struct
    /// renders without a `required` key, rather than `required: []`).
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub properties: Option<IndexMap<String, ObjectSchema>>,
    /// `allOf` composition. Used under OAS 3.0 to express
    /// "nullable reference" (`allOf: [$ref], nullable: true`), and as
    /// the wrap that lets a `description` sit alongside a `$ref` (since
    /// OAS 3.0 ignores `$ref` siblings).
    #[serde(default, rename = "allOf")]
    pub all_of: Option<Vec<ObjectSchema>>,
    /// `oneOf` composition. Used under OAS 3.1 to express
    /// "nullable reference" (`oneOf: [$ref, {type: "null"}]`).
    #[serde(default, rename = "oneOf")]
    pub one_of: Option<Vec<ObjectSchema>>,
    /// Carries the intent that this schema accepts `null` in addition to
    /// values of `ty`. The field exists irrespective of the active OAS
    /// version feature — it stores the intent only. The handwritten
    /// `Serialize` impl translates this flag into the version-appropriate
    /// YAML shape (`nullable: true` for OAS 3.0; a 2-element `type` array
    /// containing `"null"` for OAS 3.1).
    #[serde(default)]
    pub nullable: Option<bool>,
}

impl ObjectSchema {
    /// An empty object schema with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Handwritten `Serialize` impl for [`ObjectSchema`] producing the
/// canonical OAS key order:
///
/// ```text
/// $ref, type, description, format, minimum, items, required,
/// properties, allOf, oneOf, nullable
/// ```
///
/// `$ref` is mutually exclusive with most siblings: when set, only
/// `description` (and only under OAS 3.1) is emitted alongside.
///
/// The OAS 3.0 / 3.1 difference is encoded here:
///
/// - Under `oas-3-0`, a nullable schema emits `nullable: true` at the
///   tail position, and `type` is a scalar string.
/// - Under `oas-3-1`, `type` becomes the 2-element sequence
///   `[<base>, "null"]` whenever the schema is nullable; no separate
///   `nullable` key is emitted.
impl Serialize for ObjectSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;

        if let Some(reference) = &self.reference {
            map.serialize_entry("$ref", reference)?;
            serialize_reference_siblings(&mut map, self)?;
            return map.end();
        }

        serialize_type(&mut map, self)?;

        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        if let Some(format) = &self.format {
            map.serialize_entry("format", format)?;
        }
        if let Some(minimum) = self.minimum {
            map.serialize_entry("minimum", &Minimum(minimum))?;
        }
        if let Some(items) = &self.items {
            map.serialize_entry("items", items)?;
        }
        if !self.required.is_empty() {
            map.serialize_entry("required", &self.required)?;
        }
        if let Some(properties) = &self.properties {
            map.serialize_entry("properties", properties)?;
        }
        if let Some(all_of) = &self.all_of {
            map.serialize_entry("allOf", all_of)?;
        }
        if let Some(one_of) = &self.one_of {
            map.serialize_entry("oneOf", one_of)?;
        }
        serialize_nullable(&mut map, self)?;

        map.end()
    }
}

/// Emits sibling keys allowed next to `$ref` on the active OAS version.
///
/// Under OAS 3.0 nothing is emitted — `$ref` siblings are ignored on the
/// wire. Under OAS 3.1, `description` is permitted as a sibling and is
/// emitted in the canonical post-`$ref` position.
#[cfg(feature = "oas-3-0")]
fn serialize_reference_siblings<M>(_map: &mut M, _schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    Ok(())
}

#[cfg(feature = "oas-3-1")]
fn serialize_reference_siblings<M>(map: &mut M, schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    if let Some(description) = &schema.description {
        map.serialize_entry("description", description)?;
    }
    Ok(())
}

/// Emits the `type` key.
///
/// Under `oas-3-0`, `type` is always a scalar string (nullability is
/// emitted separately by [`serialize_nullable`]).
///
/// Under `oas-3-1`, `type` becomes a 2-element sequence
/// `[<base>, "null"]` when the schema is nullable. The `"null"` is the
/// distinct [`SchemaType::Null`] variant, which serializes to the string
/// `"null"`.
#[cfg(feature = "oas-3-0")]
fn serialize_type<M>(map: &mut M, schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    if let Some(ty) = schema.ty {
        map.serialize_entry("type", &ty)?;
    }
    Ok(())
}

#[cfg(feature = "oas-3-1")]
fn serialize_type<M>(map: &mut M, schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    if let Some(ty) = schema.ty {
        if schema.nullable == Some(true) {
            map.serialize_entry("type", &TypeWithNull(ty))?;
        } else {
            map.serialize_entry("type", &ty)?;
        }
    }
    Ok(())
}

/// Emits the nullability marker.
///
/// Under `oas-3-0`, a nullable schema gets `nullable: true`. Under
/// `oas-3-1`, the nullability marker is folded into `type` and this
/// function emits nothing.
#[cfg(feature = "oas-3-0")]
fn serialize_nullable<M>(map: &mut M, schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    if schema.nullable == Some(true) {
        map.serialize_entry("nullable", &true)?;
    }
    Ok(())
}

#[cfg(feature = "oas-3-1")]
fn serialize_nullable<M>(_map: &mut M, _schema: &ObjectSchema) -> Result<(), M::Error>
where
    M: SerializeMap,
{
    Ok(())
}

/// `Serialize` wrapper for the `minimum` field that emits a whole-number
/// bound as an integer (the OAS-idiomatic `minimum: 0` rather than
/// `0.0`), and any fractional bound as a float.
///
/// `ObjectSchema.minimum` is typed as `f64` to leave room for fractional
/// bounds, but the only values the derive currently produces are integer
/// constants (`0` for `u32` / `u64`), which should render as integers on
/// the wire.
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
#[cfg(feature = "oas-3-1")]
struct TypeWithNull(SchemaType);

#[cfg(feature = "oas-3-1")]
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
