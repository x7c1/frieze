//! The "string enum schema" half of the [`crate::SchemaObject`] sum.
//!
//! Renders as `{type: string, description?, enum: [...]}` in YAML. The
//! canonical key order within a string-enum schema is
//! `type, description, enum` — `description` is emitted only when present.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

use crate::schema_type::SchemaType;

/// The string-enum variant carried by [`crate::SchemaObject`].
///
/// Values are stored in source order. Sorting is intentionally not
/// performed — the on-the-wire string representation produced by serde
/// uses source order, and matching that here keeps the OAS schema and
/// the serialised form aligned.
///
/// `Deserialize` is auto-derived solely to let this type ride along
/// inside the round-tripped [`crate::Document`]. The matching
/// `Serialize` impl is handwritten further down to produce the canonical
/// OAS `{type, description?, enum}` key order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StringEnumSchema {
    pub values: Vec<String>,
    /// Free-form description text. Carried verbatim from `frieze-model`
    /// — composition of per-variant docs happens in `frieze-macros`
    /// before reaching this side, so the value here is the final
    /// rendered string.
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

/// Handwritten `Serialize` impl producing the canonical OAS key order
/// `type, description, enum` (with `description` emitted only when
/// present). Variant values are emitted in source order — sorting is
/// deliberately avoided so the on-the-wire string form produced by
/// serde matches the OAS schema.
impl Serialize for StringEnumSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", &SchemaType::String)?;
        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        map.serialize_entry("enum", &self.values)?;
        map.end()
    }
}
