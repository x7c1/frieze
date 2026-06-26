//! The enum of property types supported in Phase 1.

use crate::schema_name::SchemaName;

/// Property types currently supported by the derive in Phase 1.
///
/// Unsigned variants (`UInt32`, `UInt64`) carry their non-negative
/// semantics over to OAS via `minimum: 0`, since OAS 3.0 has no
/// canonical unsigned representation.
///
/// `Array` and `Nullable` are recursive (each wrapping any
/// `PropertyType`), allowing the enum to express composite shapes such as
/// arrays of nullable scalars (`Array(Nullable(...))`). The derive
/// currently only constructs a single level of `Array` — nested arrays
/// (`Vec<Vec<T>>`) are syntactically expressible at the enum level but
/// rejected by the macro before reaching this type.
///
/// **Nullability is encoded on the type, not on the outer `Property`.**
/// This is the structural change introduced by PR-F that unlocks
/// per-element nullability for arrays (`Vec<Option<T>>` →
/// `Array(Nullable(...))`) and keeps presence (`required`) cleanly
/// orthogonal to value-level nullability.
///
/// [`PropertyType::Reference`] carries a [`SchemaName`] pointing at another
/// schema registered in the surrounding [`crate::Schemas`] — this is how
/// nested struct fields are expressed. Resolution (`name exists in the
/// collection`) is enforced by `SchemasBuilder::build` in
/// `frieze-usecase`, not by this enum: the value here only records *that*
/// a reference was requested.
///
/// `Copy` is intentionally NOT derived because `Array(Box<PropertyType>)`
/// and `Nullable(Box<PropertyType>)` own heap memory.
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
    /// Marks the wrapped type as accepting `null` in addition to its base
    /// values. Rendered as `nullable: true` under `oas-3-0` or as a
    /// 2-element `type` sequence under `oas-3-1`, applied at the position
    /// where this variant appears in the tree (so `Array(Nullable(...))`
    /// makes the array **items** nullable, not the array itself).
    Nullable(Box<PropertyType>),
    /// A reference to another schema registered in the same
    /// [`crate::Schemas`] collection. Rendered as
    /// `$ref: "#/components/schemas/<name>"` (non-nullable) or wrapped in
    /// `allOf` / `oneOf` when nullable, per the active OAS version.
    Reference(SchemaName),
}
