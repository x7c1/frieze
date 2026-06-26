//! The `type` field of an OpenAPI Schema Object.

use serde::Serialize;

/// The `type` field of an OpenAPI Schema Object.
///
/// `Null` is only meaningful under OAS 3.1, where the spec accepts
/// `"null"` as a primitive type name. The renderer in `frieze-usecase`
/// uses it as the second element of `oneOf` when expressing a
/// "nullable reference" (`oneOf: [$ref, {type: "null"}]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    Integer,
    Number,
    String,
    Boolean,
    Array,
    Null,
}
