//! The top-level OpenAPI Schema Object sum supported by the current derive.

use crate::object_schema::ObjectSchema;
use crate::one_of_schema::OneOfSchema;
use crate::string_enum_schema::StringEnumSchema;

/// The kinds of OAS Schema Object that frieze can register under
/// `#/components/schemas`.
///
/// The variant set determines what shapes a registered schema entry can
/// take. Per-property sub-schemas (the values inside `properties`,
/// `items`, `allOf`, `oneOf`) stay as plain [`ObjectSchema`] because OAS
/// describes them with the same key set as object-typed schemas — adding
/// new top-level shapes does not change the per-property emitter.
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
    /// A `type: string` schema whose `enum` array enumerates the allowed
    /// values. Derived from a Rust enum whose variants are all unit
    /// variants.
    StringEnum(StringEnumSchema),
    /// A `oneOf` schema with a top-level `discriminator` block, derived
    /// from an internally-tagged Rust enum (one whose every variant is
    /// a newtype wrapping a `Schema`-implementing struct, declared with
    /// `#[serde(tag = "...")]`).
    OneOf(OneOfSchema),
}
