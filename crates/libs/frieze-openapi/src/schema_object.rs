//! The top-level OpenAPI Schema Object sum supported by Phase 1.

use crate::object_schema::ObjectSchema;

/// The kinds of OAS Schema Object that frieze can register under
/// `#/components/schemas`.
///
/// The variant set determines what shapes a [`crate::ObjectSchema`]'s
/// surrounding entry can take. Per-property sub-schemas (the values inside
/// `properties`, `items`, `allOf`, `oneOf`) stay as plain [`ObjectSchema`]
/// because OAS describes them with the same key set as object-typed
/// schemas — adding new top-level shapes (e.g. `oneOf` as a discriminated
/// union later) does not change the per-property emitter.
///
/// Matches on this sum are intentionally exhaustive across the crate:
/// adding a variant should surface a compile error at every emission site.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaObject {
    /// A standard object schema — `type: object` with `properties` and
    /// `required` derived from the source struct's fields, or any
    /// per-property shape (`$ref`, scalar `type` + `format`, array,
    /// nullable composition).
    Object(ObjectSchema),
}
