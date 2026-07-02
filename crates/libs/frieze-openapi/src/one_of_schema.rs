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

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};

/// One arm of an internally-tagged [`OneOfSchema`].
///
/// `inner_reference` is the JSON pointer used as the `$ref` target in the
/// `allOf` arm — pre-formatted as `#/components/schemas/<Name>` by the
/// caller. `wire_name` is the tag value emitted in the synthesized
/// `enum: [<wire_name>]` constraint inside the same `allOf` arm.
///
/// The auto-derived `Serialize` / `Deserialize` here are only used to
/// carry this type as data inside the round-tripped [`crate::Document`];
/// the canonical OAS arm rendering (the `allOf: [{$ref}, {tag-property
/// object}]` shape) is produced by the handwritten `Serialize` impl on
/// [`OneOfSchema`], which composes each arm itself rather than calling
/// `Serialize` on this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
///
/// `Deserialize` is auto-derived (used only for round-tripping through
/// [`crate::Document`]); the matching `Serialize` impl is handwritten
/// further down to produce the canonical OAS `oneOf` shape.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

/// Handwritten `Serialize` impl producing the canonical OAS key order
/// `description, oneOf, discriminator` (with `description` emitted only
/// when present).
///
/// Each `oneOf` arm renders as
/// `{allOf: [{$ref: <inner_reference>}, {type: object, required: [<tag>],
/// properties: {<tag>: {type: string, enum: [<wire_name>]}}}]}` so that
/// readers must shape-match both the inner struct schema and the
/// discriminator-property constraint. The enclosing schema's
/// `discriminator` block only carries `propertyName` — the optional
/// `mapping` block is deliberately omitted (a `mapping` that pointed at
/// `inner_reference` would let a reader bypass the `enum: [<wire_name>]`
/// constraint by validating the payload against the inner schema alone).
impl Serialize for OneOfSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        if let Some(description) = &self.description {
            map.serialize_entry("description", description)?;
        }
        map.serialize_entry("oneOf", &OneOfArms { schema: self })?;
        map.serialize_entry("discriminator", &Discriminator { tag: &self.tag })?;
        map.end()
    }
}

/// Adapter that serializes the variant list as a sequence of synthesized
/// `allOf` arms. Lives behind a small wrapper so the `Serialize` impl on
/// [`OneOfSchema`] can hand it to `serialize_entry`.
struct OneOfArms<'a> {
    schema: &'a OneOfSchema,
}

impl Serialize for OneOfArms<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.schema.variants.len()))?;
        for variant in &self.schema.variants {
            seq.serialize_element(&OneOfArm {
                tag: &self.schema.tag,
                variant,
            })?;
        }
        seq.end()
    }
}

/// One `allOf` arm — `[{$ref}, {synthetic tag-property object}]`.
struct OneOfArm<'a> {
    tag: &'a str,
    variant: &'a OneOfVariant,
}

impl Serialize for OneOfArm<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut arm = serializer.serialize_map(Some(1))?;
        arm.serialize_entry(
            "allOf",
            &AllOfPair {
                tag: self.tag,
                variant: self.variant,
            },
        )?;
        arm.end()
    }
}

struct AllOfPair<'a> {
    tag: &'a str,
    variant: &'a OneOfVariant,
}

impl Serialize for AllOfPair<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&InnerRef {
            inner_reference: &self.variant.inner_reference,
        })?;
        seq.serialize_element(&TagPropertyObject {
            tag: self.tag,
            wire_name: &self.variant.wire_name,
        })?;
        seq.end()
    }
}

struct InnerRef<'a> {
    inner_reference: &'a str,
}

impl Serialize for InnerRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("$ref", self.inner_reference)?;
        map.end()
    }
}

/// Renders the synthetic discriminator arm:
/// `{type: object, required: [<tag>], properties: {<tag>: {type: string,
/// enum: [<wire_name>]}}}`.
struct TagPropertyObject<'a> {
    tag: &'a str,
    wire_name: &'a str,
}

impl Serialize for TagPropertyObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("type", &crate::schema_type::SchemaType::Object)?;
        map.serialize_entry("required", &[self.tag])?;
        map.serialize_entry(
            "properties",
            &TagProperties {
                tag: self.tag,
                wire_name: self.wire_name,
            },
        )?;
        map.end()
    }
}

struct TagProperties<'a> {
    tag: &'a str,
    wire_name: &'a str,
}

impl Serialize for TagProperties<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(
            self.tag,
            &TagPropertyConstraint {
                wire_name: self.wire_name,
            },
        )?;
        map.end()
    }
}

struct TagPropertyConstraint<'a> {
    wire_name: &'a str,
}

impl Serialize for TagPropertyConstraint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", &crate::schema_type::SchemaType::String)?;
        map.serialize_entry("enum", &[self.wire_name])?;
        map.end()
    }
}

struct Discriminator<'a> {
    tag: &'a str,
}

impl Serialize for Discriminator<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("propertyName", self.tag)?;
        map.end()
    }
}
