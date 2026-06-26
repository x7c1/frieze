//! The OpenAPI Schema Object subset supported by Phase 1.

use indexmap::IndexMap;
use serde::Serialize;

use crate::schema_type::SchemaType;

/// A subset of the OpenAPI Schema Object sufficient for Phase 1.
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
///
/// Field declaration order matches the canonical YAML output order
/// (`$ref`, `type`, `items`, `format`, `minimum`, `allOf`, `oneOf`,
/// `nullable`, `properties`, `required`).
///
/// A schema object set to a `$ref` is, per OAS, a leaf — when `reference`
/// is set, callers must not also set sibling fields. The renderer in
/// `frieze-usecase` honours this by emitting `$ref` alone when present.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SchemaObject {
    /// JSON pointer to another schema (typically
    /// `#/components/schemas/<name>`). When set, the schema object is a
    /// pure reference: any sibling fields are ignored on the wire.
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<SchemaType>,
    /// Element schema for array types. Boxed because [`SchemaObject`]
    /// references itself recursively here (an array's items are themselves
    /// schema objects).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Inclusive lower bound for numeric values. Currently used only to
    /// encode Rust's unsigned semantics in OAS (`minimum: 0` for `u32` /
    /// `u64`), since OAS 3.0 has no canonical unsigned representation.
    ///
    /// Stored as `f64` so the type can represent fractional bounds in the
    /// future. Whole-number values are emitted as YAML integers (e.g. `0`
    /// rather than `0.0`); see `to_value` in `frieze-usecase` for the
    /// emission detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// `allOf` composition. Used under OAS 3.0 to express
    /// "nullable reference" (`allOf: [$ref], nullable: true`).
    #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<SchemaObject>>,
    /// `oneOf` composition. Used under OAS 3.1 to express
    /// "nullable reference" (`oneOf: [$ref, {type: "null"}]`).
    #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<SchemaObject>>,
    /// Carries the intent that this schema accepts `null` in addition to
    /// values of `ty`. The field exists irrespective of the active OAS
    /// version feature — it stores the intent only. The renderer in
    /// `frieze-usecase` translates this flag into the version-appropriate
    /// YAML shape (`nullable: true` for OAS 3.0; a 2-element `type` array
    /// containing `"null"` for OAS 3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, SchemaObject>>,
    /// Names of properties that must appear on the wire. Omitted from
    /// the emitted schema entirely when empty (an all-optional struct
    /// renders without a `required` key, rather than `required: []`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl SchemaObject {
    /// An empty schema object with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}
