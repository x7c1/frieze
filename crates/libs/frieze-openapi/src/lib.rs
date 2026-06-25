//! Plain Rust representation of the OpenAPI Specification.
//!
//! This crate intentionally has no knowledge of `frieze-model` or
//! `frieze-usecase`. It mirrors the shape of the OAS spec and nothing more.

use indexmap::IndexMap;
use serde::Serialize;

/// The `type` field of an OpenAPI Schema Object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Integer,
    String,
}

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
