//! The enum of property types supported in Phase 1.

/// Property types currently supported by the derive in Phase 1.
///
/// Unsigned variants (`UInt32`, `UInt64`) carry their non-negative
/// semantics over to OAS via `minimum: 0`, since OAS 3.0 has no
/// canonical unsigned representation.
///
/// `Array` is recursive (wrapping any `PropertyType`), allowing the enum
/// to represent arrays of supported scalars. The derive currently only
/// constructs a single level of `Array` — nested arrays (`Vec<Vec<T>>`)
/// are syntactically expressible at the enum level but rejected by the
/// macro before reaching this type.
///
/// `Copy` is intentionally NOT derived because `Array(Box<PropertyType>)`
/// owns heap memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyType {
    /// Maps to OpenAPI `type: integer, format: int32`.
    Int32,
    /// Maps to OpenAPI `type: integer, format: int64`.
    Int64,
    /// Maps to OpenAPI `type: integer, format: int32, minimum: 0`.
    UInt32,
    /// Maps to OpenAPI `type: integer, format: int64, minimum: 0`.
    UInt64,
    /// Maps to OpenAPI `type: number, format: float`.
    Float,
    /// Maps to OpenAPI `type: number, format: double`.
    Double,
    /// Maps to OpenAPI `type: string` (no format).
    String,
    /// Maps to OpenAPI `type: boolean` (no format).
    Boolean,
    /// Maps to OpenAPI `type: array` with `items` describing the element
    /// schema.
    Array(Box<PropertyType>),
}
