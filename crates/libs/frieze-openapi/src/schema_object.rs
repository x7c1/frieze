//! The top-level OpenAPI Schema Object sum supported by the current derive.

use serde::{Deserialize, Serialize, Serializer};

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
///
/// # Serde representation
///
/// `#[serde(untagged)]` is used so that the variant is recovered from the
/// shape of the input. The variant order below is also the deserializer's
/// trial order — most-specific first:
///
/// - [`SchemaObject::OneOf`] — chosen when the payload has the required
///   `tag` and `variants` fields of an internally-tagged `oneOf` arm.
/// - [`SchemaObject::StringEnum`] — chosen when the payload has the
///   required `values` field.
/// - [`SchemaObject::Object`] — the catch-all. Every other shape — and
///   the empty object `{}` — round-trips as this variant.
///
/// This is **not** a robust general OAS deserializer: arbitrary OAS YAML
/// that happens to mention a `tag` / `variants` / `values` field at the
/// schema level may be claimed by the wrong variant. The intent here is
/// only to let frieze-produced documents survive a YAML/JSON round-trip
/// through [`crate::OasDocument`]. Full external-input robustness will
/// arrive with the dedicated canonical (de)serializers in a later step.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SchemaObject {
    /// A `oneOf` schema with a top-level `discriminator` block, derived
    /// from an internally-tagged Rust enum (one whose every variant is
    /// a newtype wrapping a `Schema`-implementing struct, declared with
    /// `#[serde(tag = "...")]`).
    OneOf(OneOfSchema),
    /// A `type: string` schema whose `enum` array enumerates the allowed
    /// values. Derived from a Rust enum whose variants are all unit
    /// variants.
    StringEnum(StringEnumSchema),
    /// A standard object schema — `type: object` with `properties` and
    /// `required` derived from the source struct's fields, or any
    /// per-property shape (`$ref`, scalar `type` + `format`, array,
    /// nullable composition).
    Object(ObjectSchema),
}

/// Dispatch-only `Serialize` impl: delegates straight to the active
/// variant's handwritten serializer so each variant controls its own
/// canonical OAS key order.
impl Serialize for SchemaObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SchemaObject::OneOf(one_of) => one_of.serialize(serializer),
            SchemaObject::StringEnum(string_enum) => string_enum.serialize(serializer),
            SchemaObject::Object(object) => object.serialize(serializer),
        }
    }
}
