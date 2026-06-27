//! The top-level [`Schema`] sum stored in [`crate::Schemas`].

use crate::error::Error;
use crate::object_schema::ObjectSchema;
use crate::property::Property;
use crate::schema_name::SchemaName;

/// A validated domain schema.
///
/// The sum determines what shape a registered schema entry can take.
/// Variants are added as new top-level kinds are supported by the
/// derive — for Phase 1, an object schema is the only kind. Matches on
/// this sum are intentionally exhaustive across the workspace so adding
/// a variant surfaces a compile error at every consumption site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schema {
    /// A standard ("object-typed") schema with at least one property.
    Object(ObjectSchema),
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

    /// The name under which this schema is registered in
    /// [`crate::Schemas`].
    pub fn name(&self) -> &SchemaName {
        match self {
            Schema::Object(o) => &o.name,
        }
    }
}
