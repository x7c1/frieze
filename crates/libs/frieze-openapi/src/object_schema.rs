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
use serde::{Deserialize, Serialize};

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
/// The `#[derive(Serialize, Deserialize)]` impls here are an intentionally
/// naive auto-derive used only as a transport: they let `ObjectSchema`
/// values move through the new top-level [`crate::OasDocument`] /
/// [`crate::Components`] structs (round-tripped via `serde_yaml` /
/// `serde_json`) without hand-writing their conversions. They are NOT the
/// canonical, key-ordered OAS rendering — that is still produced by
/// `frieze-usecase::to_value` / `to_yaml`, which build the wire shape
/// manually and do not call `Serialize` on this type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectSchema {
    /// JSON pointer to another schema (typically
    /// `#/components/schemas/<name>`). When set, the schema object is a
    /// pure reference: any sibling fields are ignored on the wire.
    #[serde(default, rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<SchemaType>,
    /// Free-form description text. Rendered as the `description` field
    /// of the OAS schema when present. Empty / whitespace-only inputs
    /// are stripped to `None` upstream in `frieze-model` so this side
    /// only ever sees a meaningful value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Inclusive lower bound for numeric values. Currently used only to
    /// encode Rust's unsigned semantics in OAS (`minimum: 0` for `u32` /
    /// `u64`), since OAS 3.0 has no canonical unsigned representation.
    ///
    /// Stored as `f64` so the type can represent fractional bounds in the
    /// future. Whole-number values are emitted as YAML integers (e.g. `0`
    /// rather than `0.0`); see `to_value` in `frieze-usecase` for the
    /// emission detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Element schema for array types. Boxed because [`ObjectSchema`]
    /// references itself recursively here (an array's items are themselves
    /// schema objects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ObjectSchema>>,
    /// Names of properties that must appear on the wire. Omitted from
    /// the emitted schema entirely when empty (an all-optional struct
    /// renders without a `required` key, rather than `required: []`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, ObjectSchema>>,
    /// `allOf` composition. Used under OAS 3.0 to express
    /// "nullable reference" (`allOf: [$ref], nullable: true`), and as
    /// the wrap that lets a `description` sit alongside a `$ref` (since
    /// OAS 3.0 ignores `$ref` siblings).
    #[serde(default, rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<ObjectSchema>>,
    /// `oneOf` composition. Used under OAS 3.1 to express
    /// "nullable reference" (`oneOf: [$ref, {type: "null"}]`).
    #[serde(default, rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<ObjectSchema>>,
    /// Carries the intent that this schema accepts `null` in addition to
    /// values of `ty`. The field exists irrespective of the active OAS
    /// version feature — it stores the intent only. The renderer in
    /// `frieze-usecase` translates this flag into the version-appropriate
    /// YAML shape (`nullable: true` for OAS 3.0; a 2-element `type` array
    /// containing `"null"` for OAS 3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
}

impl ObjectSchema {
    /// An empty object schema with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}
