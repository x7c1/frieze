//! The top-level OpenAPI Schema Object sum supported by the current derive.

use serde::{Deserialize, Serialize};

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
/// The derived `Serialize` / `Deserialize` here are the canonical
/// (version-neutral) form: `#[serde(untagged)]` serializes the active
/// variant transparently through its own derived (canonical) impl, and
/// recovers the variant from the shape of the input. The OAS wire form
/// is emitted by the versioned dispatcher in the `serialize` module,
/// which [`crate::Document`] routes through automatically.
///
/// The variant order below is the deserializer's trial order —
/// most-specific first:
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
/// through [`crate::Document`]. Full external-input robustness will
/// arrive with dedicated canonical (de)serializers in a later step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
