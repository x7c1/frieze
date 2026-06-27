//! The top-level [`Schema`] sum stored in [`crate::Schemas`].

use crate::error::Error;
use crate::object_schema::ObjectSchema;
use crate::property::Property;
use crate::schema_name::SchemaName;
use crate::string_enum_schema::StringEnumSchema;

/// A validated domain schema.
///
/// The sum determines what shape a registered schema entry can take.
/// Variants are added as new top-level kinds are supported by the
/// derive — Phase 1 covers object schemas and unit-variant enum
/// schemas; richer enum shapes (data-carrying variants, `oneOf`) will
/// arrive as further variants. Matches on this sum are intentionally
/// exhaustive across the workspace so adding a variant surfaces a
/// compile error at every consumption site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schema {
    /// A standard ("object-typed") schema with at least one property.
    Object(ObjectSchema),
    /// A `type: string, enum: [...]` schema derived from a Rust enum
    /// whose variants are all unit variants.
    StringEnum(StringEnumSchema),
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

    /// The name under which this schema is registered in
    /// [`crate::Schemas`].
    pub fn name(&self) -> &SchemaName {
        match self {
            Schema::Object(o) => &o.name,
            Schema::StringEnum(e) => &e.name,
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
        }
    }
}
