//! The "string enum schema" half of the [`crate::SchemaObject`] sum.
//!
//! Renders as `{type: string, enum: [...]}` in YAML. The canonical key
//! order within a string-enum schema is `type, enum`.

/// The string-enum variant carried by [`crate::SchemaObject`].
///
/// Values are stored in source order. Sorting is intentionally not
/// performed — the on-the-wire string representation produced by serde
/// uses source order, and matching that here keeps the OAS schema and
/// the serialised form aligned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEnumSchema {
    pub values: Vec<String>,
}

impl StringEnumSchema {
    /// Builds a string-enum schema from a pre-validated values list.
    ///
    /// Validation (non-empty, distinct, non-empty entries) is the
    /// responsibility of the caller — for the derive path, that's the
    /// `frieze-model` constructor on `StringEnumSchema` in that crate.
    pub fn new(values: Vec<String>) -> Self {
        Self { values }
    }
}
