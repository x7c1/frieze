//! The "string enum schema" half of the [`crate::SchemaObject`] sum.
//!
//! On the OAS wire this renders as `{type: string, description?,
//! enum: [...]}` in YAML. The canonical key order within a string-enum
//! schema is `type, description, enum` — `description` is emitted only
//! when present.

use serde::{Deserialize, Serialize};

/// The string-enum variant carried by [`crate::SchemaObject`].
///
/// Values are stored in source order. Sorting is intentionally not
/// performed — the on-the-wire string representation produced by serde
/// uses source order, and matching that here keeps the OAS schema and
/// the serialised form aligned.
///
/// The derived `Serialize` / `Deserialize` are the canonical
/// (version-neutral) form — `{values, description?}`, mirroring the
/// struct fields — used for machine-readable dumps and round-tripping
/// through [`crate::Document`]. The OAS `{type, description?, enum}`
/// wire shape is produced by the versioned emitter in the `serialize`
/// module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringEnumSchema {
    pub values: Vec<String>,
    /// Free-form description text. Carried verbatim from `frieze-model`
    /// — composition of per-variant docs happens in `frieze-macros`
    /// before reaching this side, so the value here is the final
    /// rendered string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl StringEnumSchema {
    /// Builds a string-enum schema from a pre-validated values list.
    ///
    /// Validation (non-empty, distinct, non-empty entries) is the
    /// responsibility of the caller — for the derive path, that's the
    /// `frieze-model` constructor on `StringEnumSchema` in that crate.
    /// The description is initialized to `None`; use
    /// [`StringEnumSchema::with_description`] to attach one.
    pub fn new(values: Vec<String>) -> Self {
        Self {
            values,
            description: None,
        }
    }

    /// Attaches a description to the schema. The caller is responsible
    /// for normalizing empty input — this side trusts what it receives.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }
}
