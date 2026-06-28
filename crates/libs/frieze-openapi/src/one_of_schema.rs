//! The "internally-tagged `oneOf` schema" arm of the [`crate::SchemaObject`]
//! sum.
//!
//! Renders, per arm, as
//! `allOf: [ {$ref: <inner>}, {type: object, required: [<tag>], properties: {<tag>: {type: string, enum: [<wire_name>]}}} ]`,
//! with a sibling `discriminator: {propertyName: <tag>}` block on the
//! enclosing schema. The `discriminator.mapping` block is deliberately
//! omitted — see the rationale in the rendering module of
//! `frieze-usecase`. The canonical key order within a `oneOf` schema is
//! `description, oneOf, discriminator` (description only when present).

/// One arm of an internally-tagged [`OneOfSchema`].
///
/// `inner_reference` is the JSON pointer used as the `$ref` target in the
/// `allOf` arm — pre-formatted as `#/components/schemas/<Name>` by the
/// caller. `wire_name` is the tag value emitted in the synthesized
/// `enum: [<wire_name>]` constraint inside the same `allOf` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOfVariant {
    pub wire_name: String,
    pub inner_reference: String,
}

/// The "oneOf schema" variant carried by [`crate::SchemaObject`].
///
/// `tag` is the discriminator property name (`#[serde(tag = "...")]`).
/// `variants` lists each arm in source declaration order. `description`
/// already carries the composed enum-level-plus-per-variant doc text
/// produced upstream in `frieze-macros`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOfSchema {
    pub tag: String,
    pub variants: Vec<OneOfVariant>,
    pub description: Option<String>,
}

impl OneOfSchema {
    /// Builds a `oneOf` schema from a pre-validated variants list and
    /// non-empty tag. Validation (non-empty tag, distinct non-empty
    /// `wire_name`s) is the responsibility of the caller — for the derive
    /// path, that's the `OneOfSchema` constructor in `frieze-model`. The
    /// description is initialized to `None`; use
    /// [`OneOfSchema::with_description`] to attach one.
    pub fn new(tag: impl Into<String>, variants: Vec<OneOfVariant>) -> Self {
        Self {
            tag: tag.into(),
            variants,
            description: None,
        }
    }

    /// Attaches a description to the schema. The caller is responsible
    /// for normalising empty input — this side trusts what it receives.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }
}
