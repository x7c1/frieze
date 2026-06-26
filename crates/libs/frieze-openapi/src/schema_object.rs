//! The OpenAPI Schema Object subset supported by Phase 1.

use indexmap::IndexMap;
use serde::Serialize;

use crate::schema_type::SchemaType;

/// A subset of the OpenAPI Schema Object sufficient for Phase 1.
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
///
/// Field declaration order matches the canonical YAML output order
/// (`type`, `format`, `minimum`, `nullable`, `properties`, `required`):
/// `type` first, then `format`, then numeric constraints, then the
/// nullability marker, then container fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SchemaObject {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<SchemaType>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl SchemaObject {
    /// An empty schema object with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}
