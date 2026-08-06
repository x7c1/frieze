//! The idempotent push primitive every registration path funnels
//! through, and the deferred-error bookkeeping it performs.

use frieze_model::{primitive_property_type_for, Error, Schema as ModelSchema};

use super::SchemasBuilder;

impl SchemasBuilder {
    /// Pushes `schema` into the in-progress collection only if no schema
    /// with the same registration name is already present.
    ///
    /// `Schema::Scalar` entries (and anything else whose
    /// [`ModelSchema::name`] returns `None`) are always appended — they
    /// have no key to dedup on and are filtered out at
    /// [`Schemas::new`](frieze_model::Schemas::new) anyway.
    ///
    /// This is the idempotent push primitive used by
    /// [`Register::register_into`](crate::Register::register_into) (both
    /// the default impl and the derive-emitted override) so the same
    /// root reached through multiple paths or via a self-referential
    /// cycle (`struct Tree { children: Vec<Box<Tree>> }`) collapses to a
    /// single entry.
    ///
    /// When an incoming schema shares a name with one already registered
    /// but their bodies differ, the first such conflict is recorded on
    /// the builder and reported as [`Error::SchemaConflict`] from
    /// [`Self::build`]. Same-name / same-content pushes remain a silent
    /// dedup — the normal case for a transitive root reached through
    /// multiple paths.
    ///
    /// A name that exactly matches one of the nine primitive scalar
    /// names ([`primitive_property_type_for`]) is likewise recorded and
    /// reported as [`Error::ReservedSchemaName`]. Every registration
    /// path (`add::<T>()`, transitive `register_into`, inventory
    /// collection) funnels through here, so hooking the check at this
    /// point covers all of them. The set is feature-independent:
    /// `Uuid` is reserved even when the `uuid1` feature is off, because
    /// the boundary inlines a `Reference("Uuid")` either way.
    pub fn push_unique(&mut self, schema: ModelSchema) {
        let Some(name) = schema.name().cloned() else {
            // Schemas with no registration name (e.g. `Schema::Scalar`)
            // have no key to dedup on; preserve the historical
            // append-always behaviour. `Schemas::new` filters them out
            // afterwards anyway.
            self.schemas.push(schema);
            return;
        };
        // A reserved name is recorded, but the entry is still pushed:
        // `build` fails regardless, and dropping it here would defeat the
        // `contains_name` guard the derived `register_into` uses to break
        // recursion through a self-referential type.
        if primitive_property_type_for(&name).is_some() && self.reserved.is_none() {
            self.reserved = Some(Error::ReservedSchemaName { name: name.clone() });
        }
        if let Some(existing) = self.schemas.iter().find(|s| s.name() == Some(&name)) {
            if existing != &schema && self.conflict.is_none() {
                self.conflict = Some(Error::SchemaConflict {
                    name,
                    existing: Box::new(existing.clone()),
                    incoming: Box::new(schema),
                });
            }
            // Either way, do not push: a same-name entry is already
            // present, and recording a second copy would defeat the
            // dedup invariant `register_into` relies on.
            return;
        }
        self.schemas.push(schema);
    }
}
