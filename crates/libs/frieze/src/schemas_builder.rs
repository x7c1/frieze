//! Builder that collects [`Schema`](crate::Schema) implementations into
//! a validated [`frieze_model::Schemas`].
//!
//! The root holds the [`SchemasBuilder`] type, `new`, and `add`; this
//! module's other methods each live in the sibling module named after
//! them, and [`reference_checks`] holds the walks that `build` runs.

mod build;
mod contains_name;
mod push_unique;
mod reference_checks;

#[cfg(test)]
mod testing;

use frieze_model::{Error, Schema as ModelSchema};

use crate::register::{IsRegistrable, Register};

/// In-progress collection of schemas.
///
/// `reserved` holds the first name that collides with a primitive
/// scalar name and surfaces as [`Error::ReservedSchemaName`];
/// `conflict` holds the first same-name / different-content collision
/// and surfaces as [`Error::SchemaConflict`]. Both are observed by
/// [`SchemasBuilder::push_unique`] and reported from
/// [`SchemasBuilder::build`]: the builder follows the same
/// fail-at-finalization pattern as [`Error::UnresolvedReference`] so
/// `register_into` callers and the derive expansion can keep pushing
/// through `push_unique` without threading a `Result` through every
/// registration site.
#[derive(Debug, Default)]
pub struct SchemasBuilder {
    schemas: Vec<ModelSchema>,
    reserved: Option<Error>,
    conflict: Option<Error>,
}

impl SchemasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()` and recursively
    /// registers every type that `T`'s schema references.
    ///
    /// `T` must implement [`IsRegistrable`] — this rejects primitive
    /// scalars at compile time (`Schemas::add::<i64>()` fails to
    /// compile), since primitive scalars implement [`crate::Schema`] /
    /// [`Register`] only so they can appear as generic arguments and
    /// are not standalone OAS schema entries. `#[derive(Schema)]` emits
    /// the `IsRegistrable` impl for struct and enum inputs.
    ///
    /// The traversal is performed by [`Register::register_into`]: the
    /// derived impl walks each field type's `register_into`, so a single
    /// `add::<Foo>()` call pulls in `Foo` together with every nested
    /// struct / enum / generic instance reachable from `Foo`'s fields.
    /// Calls are idempotent — adding the same root twice, or having a
    /// root reachable through multiple paths, leaves only one entry per
    /// name in the resulting `Schemas`.
    //
    // Keep this method in the module root: the `frieze-macros` UI test
    // `add_primitive_rejected` pins this file's path in the "required by
    // a bound in `SchemasBuilder::add`" note, so moving it into a
    // sibling module would break that expectation.
    pub fn add<T: IsRegistrable>(mut self) -> Self {
        <T as Register>::register_into(&mut self);
        self
    }
}
