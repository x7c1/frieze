//! The enum of primitive scalar types supported in Phase 1.

/// Primitive scalar types currently supported by the derive in Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    /// Maps to OpenAPI `type: integer, format: int64`.
    Int64,
    /// Maps to OpenAPI `type: string` (no format).
    String,
    /// Maps to OpenAPI `type: boolean` (no format).
    Boolean,
}
