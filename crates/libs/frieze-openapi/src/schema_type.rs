//! The `type` field of an OpenAPI Schema Object.

use serde::Serialize;

/// The `type` field of an OpenAPI Schema Object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Integer,
    Number,
    String,
    Boolean,
    Array,
}
