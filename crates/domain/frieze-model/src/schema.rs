//! The top-level [`Schema`] sum stored in [`crate::Schemas`].

use crate::error::Error;
use crate::object_schema::ObjectSchema;
use crate::one_of_schema::{OneOfSchema, OneOfVariant};
use crate::property::Property;
use crate::property_type::PropertyType;
use crate::scalar_schema::ScalarSchema;
use crate::schema_name::SchemaName;
use crate::string_enum_schema::StringEnumSchema;

/// A validated domain schema.
///
/// The sum determines what shape a registered schema entry can take.
/// Variants are added as new top-level kinds are supported by the
/// derive — Phase 1 covers object schemas, unit-variant enum schemas,
/// internally-tagged `oneOf` schemas built from enums whose every
/// variant is a newtype wrapping a `Schema`-implementing struct, and
/// scalar schemas wrapping a single leaf [`PropertyType`]. Matches on
/// this sum are intentionally exhaustive across the workspace so adding
/// a variant surfaces a compile error at every consumption site.
///
/// [`Schema::Scalar`] exists so primitive Rust types (`i32`, `String`,
/// etc.) can implement the `frieze_usecase::Schema` trait and appear as
/// generic arguments (`Box<i64>`, `Page<String>`). Scalar schemas are
/// **not** registered under `#/components/schemas` — the
/// `IsRegistrable` marker trait in `frieze-usecase` rejects
/// `Schemas::add::<i64>()` at compile time, and the boundary conversion
/// in `frieze-usecase::to_value` skips Scalar entries as a defensive
/// secondary guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schema {
    /// A standard ("object-typed") schema with at least one property.
    Object(ObjectSchema),
    /// A `type: string, enum: [...]` schema derived from a Rust enum
    /// whose variants are all unit variants.
    StringEnum(StringEnumSchema),
    /// A `oneOf` schema derived from an internally-tagged Rust enum (one
    /// whose every variant is a newtype wrapping a `Schema`-implementing
    /// struct, declared with `#[serde(tag = "...")]`).
    OneOf(OneOfSchema),
    /// A scalar schema wrapping a single leaf [`PropertyType`]. Used to
    /// give primitive Rust types a `Schema` implementation so they can
    /// appear as generic arguments; never registered under
    /// `#/components/schemas`.
    Scalar(ScalarSchema),
}

impl Schema {
    /// Builds an object-typed schema, rejecting empty names, empty
    /// property lists, and duplicate property names.
    ///
    /// Convenience wrapper around [`ObjectSchema::new`] that returns the
    /// surrounding [`Schema::Object`] variant directly so callers do not
    /// need to import [`ObjectSchema`] just to wrap.
    pub fn new_object(name: impl Into<String>, properties: Vec<Property>) -> Result<Self, Error> {
        ObjectSchema::new(name, properties).map(Schema::Object)
    }

    /// Builds a string-enum schema, rejecting empty names, empty value
    /// lists, empty value strings, and duplicate values.
    ///
    /// Convenience wrapper around [`StringEnumSchema::new`] that returns
    /// the surrounding [`Schema::StringEnum`] variant directly so callers
    /// do not need to import [`StringEnumSchema`] just to wrap.
    pub fn new_string_enum(name: impl Into<String>, values: Vec<String>) -> Result<Self, Error> {
        StringEnumSchema::new(name, values).map(Schema::StringEnum)
    }

    /// Builds a `oneOf` schema, rejecting empty names, empty tags, empty
    /// variant lists, empty variant wire names, and duplicate wire names.
    ///
    /// Convenience wrapper around [`OneOfSchema::new`] that returns the
    /// surrounding [`Schema::OneOf`] variant directly so callers do not
    /// need to import [`OneOfSchema`] just to wrap.
    pub fn new_one_of(
        name: impl Into<String>,
        tag: impl Into<String>,
        variants: Vec<OneOfVariant>,
    ) -> Result<Self, Error> {
        OneOfSchema::new(name, tag, variants).map(Schema::OneOf)
    }

    /// Builds a scalar schema, rejecting composite `PropertyType`
    /// variants (`Array`, `Nullable`, `Reference`).
    ///
    /// Convenience wrapper around [`ScalarSchema::new`] that returns the
    /// surrounding [`Schema::Scalar`] variant directly so callers do not
    /// need to import [`ScalarSchema`] just to wrap.
    pub fn new_scalar(property_type: PropertyType) -> Result<Self, Error> {
        ScalarSchema::new(property_type).map(Schema::Scalar)
    }

    /// The name under which this schema is registered in
    /// [`crate::Schemas`].
    ///
    /// Returns `None` for [`Schema::Scalar`] — scalar schemas have no
    /// registration name because they are never registered under
    /// `#/components/schemas`. Callers that iterate registered schemas
    /// (e.g. [`crate::Schemas::new`]) filter out scalar entries on this
    /// `None`.
    pub fn name(&self) -> Option<&SchemaName> {
        match self {
            Schema::Object(o) => Some(&o.name),
            Schema::StringEnum(e) => Some(&e.name),
            Schema::OneOf(o) => Some(&o.name),
            Schema::Scalar(_) => None,
        }
    }

    /// Attaches a top-level description to whichever variant this is.
    ///
    /// Convenience wrapper over the per-variant `with_description`
    /// methods so the derive expansion can chain `.with_description(...)`
    /// without matching on the sum.
    #[must_use]
    pub fn with_description(self, description: Option<String>) -> Self {
        match self {
            Schema::Object(o) => Schema::Object(o.with_description(description)),
            Schema::StringEnum(e) => Schema::StringEnum(e.with_description(description)),
            Schema::OneOf(o) => Schema::OneOf(o.with_description(description)),
            Schema::Scalar(s) => Schema::Scalar(s.with_description(description)),
        }
    }
}
