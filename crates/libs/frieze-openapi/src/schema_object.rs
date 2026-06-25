//! The OpenAPI Schema Object subset supported by Phase 1.

use indexmap::IndexMap;
use serde::Serialize;

use crate::schema_type::SchemaType;

/// A subset of the OpenAPI Schema Object sufficient for Phase 1.
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SchemaObject {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<SchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
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
