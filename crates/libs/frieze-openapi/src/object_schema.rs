//! The "object schema" half of the [`crate::SchemaObject`] sum.
//!
//! This carries every OAS Schema Object key the per-property renderer
//! produces: `$ref`, `type`, `description`, `format`, `minimum`, `items`,
//! `required`, `properties`, `allOf`, `oneOf`, `nullable`. It is used
//! both for top-level object schemas (with `properties` / `required`
//! populated) and for the inner per-property and per-`oneOf`/-`allOf`/
//! -`items` sub-schemas, which leave `properties` empty and populate
//! `ty` / `format` / etc. as needed.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::schema_type::SchemaType;

/// The "object schema" variant carried by [`crate::SchemaObject`].
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
///
/// The field order in this struct mirrors the canonical YAML output order
/// (`$ref`, `type`, `description`, `format`, `minimum`, `items`,
/// `required`, `properties`, `allOf`, `oneOf`, `nullable`) so
/// contributors can predict the output shape by reading the struct.
///
/// A schema object set to a `$ref` is a *reference schema*: `reference`
/// plus optionally `description` (doc text for the referencing
/// property) and `nullable` (the value may also be `null`). How those
/// companions appear on the OAS wire differs between 3.0 and 3.1 —
/// that translation happens at serialization time, not here.
///
/// # Canonical versus OAS wire form
///
/// The derived `Serialize` / `Deserialize` impls on this type are the
/// *canonical, version-neutral* form: fields map one-to-one to keys
/// (`$ref`, `description` and `nullable` appear as plain siblings) and
/// the output round-trips losslessly. Machine-readable dumps of
/// [`crate::Components`] use this form.
///
/// The *OAS wire form* — where the OAS 3.0 / 3.1 encoding split for
/// nullability and `$ref` siblings is applied — is produced by the
/// crate-private versioned emitter (see the `serialize` module), which
/// [`crate::Document`] routes through automatically based on its
/// `oas_version` field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectSchema {
    /// JSON pointer to another schema (typically
    /// `#/components/schemas/<name>`). When set, the schema object is a
    /// reference schema: only `description` and `nullable` are
    /// meaningful siblings, and the versioned emitter translates the
    /// trio into the shape the target OAS version allows.
    #[serde(default, rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<SchemaType>,
    /// Free-form description text. Rendered as the `description` field
    /// of the OAS schema when present. Empty / whitespace-only inputs
    /// are stripped to `None` upstream in `frieze-model` so this side
    /// only ever sees a meaningful value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Inclusive lower bound for numeric values. Currently used only to
    /// encode Rust's unsigned semantics in OAS (`minimum: 0` for `u32` /
    /// `u64`), since OAS 3.0 has no canonical unsigned representation.
    ///
    /// Stored as `f64` so the type can represent fractional bounds in
    /// the future. On the OAS wire, whole-number values are emitted as
    /// YAML integers (e.g. `0` rather than `0.0`); the versioned
    /// emitter picks the integer form whenever the bound round-trips
    /// losslessly through `i64`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Element schema for array types. Boxed because [`ObjectSchema`]
    /// references itself recursively here (an array's items are themselves
    /// schema objects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ObjectSchema>>,
    /// Names of properties that must appear on the wire. Omitted from
    /// the emitted schema entirely when empty (an all-optional struct
    /// renders without a `required` key, rather than `required: []`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, ObjectSchema>>,
    /// `allOf` composition. Appears as explicit data when a document
    /// parsed from the OAS 3.0 wire carried the "nullable reference"
    /// wrap (`allOf: [$ref], nullable: true`) or the
    /// description-next-to-`$ref` wrap. Newly constructed schemas
    /// express those intents with `reference` + `nullable` /
    /// `description` instead and let the versioned emitter synthesize
    /// the wrap.
    #[serde(default, rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<ObjectSchema>>,
    /// `oneOf` composition. Appears as explicit data when a document
    /// parsed from the OAS 3.1 wire carried the "nullable reference"
    /// shape (`oneOf: [$ref, {type: "null"}]`). Newly constructed
    /// schemas express that intent with `reference` + `nullable`
    /// instead and let the versioned emitter synthesize the shape.
    #[serde(default, rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<ObjectSchema>>,
    /// Carries the intent that this schema accepts `null` in addition
    /// to values of `ty` (or the referenced schema). The field stores
    /// the intent only; the versioned emitter translates it into the
    /// version-appropriate wire shape (`nullable: true` for OAS 3.0; a
    /// 2-element `type` array containing `"null"` — or a `oneOf`
    /// against `{type: "null"}` for references — for OAS 3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
}

impl ObjectSchema {
    /// An empty object schema with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}
