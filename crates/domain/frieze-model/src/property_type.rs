//! The enum of property types currently supported by the derive.

use crate::schema_name::SchemaName;

/// Maps a [`SchemaName`] to its primitive leaf [`PropertyType`] if the
/// name matches one of the eight primitive scalar conventions
/// (`Int32` / `Int64` / `UInt32` / `UInt64` / `Float` / `Double` /
/// `Boolean` / `String`).
///
/// Generic instantiations whose argument is a primitive (e.g.
/// `Container<i64>`) emit `PropertyType::Reference(SchemaName("Int64"))`
/// from the derive output, because the macro cannot determine whether
/// the type parameter is a primitive at expansion time. Primitives are
/// not [`crate::Schemas`] entries (they implement `Schema` but not
/// `IsRegistrable`), so such a reference would otherwise be unresolved.
///
/// This helper is the single source of truth used by:
///
/// - the build-time reference walk in `frieze-usecase`, which treats a
///   primitive-named reference as resolved without requiring a
///   registered entry;
/// - the boundary conversion in `frieze-usecase::to_value`, which
///   inlines the leaf scalar shape (`{type: integer, format: int64}`,
///   `{type: string}`, ...) at the reference position instead of
///   emitting a dangling `$ref: #/components/schemas/Int64`.
///
/// Returns `None` for any other name; the caller then falls back to its
/// normal "registered reference" treatment.
pub fn primitive_property_type_for(name: &SchemaName) -> Option<PropertyType> {
    match name.as_str() {
        "Int32" => Some(PropertyType::Int32),
        "Int64" => Some(PropertyType::Int64),
        "UInt32" => Some(PropertyType::UInt32),
        "UInt64" => Some(PropertyType::UInt64),
        "Float" => Some(PropertyType::Float),
        "Double" => Some(PropertyType::Double),
        "Boolean" => Some(PropertyType::Boolean),
        "String" => Some(PropertyType::String),
        _ => None,
    }
}

/// Property types currently supported by the derive.
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
/// Separating presence from nullability is what unlocks per-element
/// nullability for arrays (`Vec<Option<T>>` → `Array(Nullable(...))`)
/// and keeps presence (`required`) cleanly orthogonal to value-level
/// nullability.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_lookup_maps_each_scalar_name() {
        let cases: &[(&str, PropertyType)] = &[
            ("Int32", PropertyType::Int32),
            ("Int64", PropertyType::Int64),
            ("UInt32", PropertyType::UInt32),
            ("UInt64", PropertyType::UInt64),
            ("Float", PropertyType::Float),
            ("Double", PropertyType::Double),
            ("Boolean", PropertyType::Boolean),
            ("String", PropertyType::String),
        ];
        for (name, expected) in cases {
            let actual = primitive_property_type_for(&SchemaName::new(*name).unwrap());
            assert_eq!(
                actual.as_ref(),
                Some(expected),
                "primitive lookup mismatch for `{name}`"
            );
        }
    }

    #[test]
    fn primitive_lookup_returns_none_for_non_primitive_names() {
        for input in ["User", "Int64_Container", "string", "INT64", "Int128"] {
            let name = SchemaName::new(input).unwrap();
            assert!(
                primitive_property_type_for(&name).is_none(),
                "expected non-primitive name `{input}` to map to None"
            );
        }
    }
}
