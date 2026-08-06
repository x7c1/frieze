//! The name lookup the derived `register_into` uses to break recursion.

use super::SchemasBuilder;

impl SchemasBuilder {
    /// Returns `true` if a previously-pushed schema has the same
    /// registration name as `name`.
    ///
    /// The derive-emitted
    /// [`Register::register_into`](crate::Register::register_into) uses
    /// this as the early-return guard at the top of the body: `if
    /// builder.contains_name(&Self::name()) { return; }` short-circuits
    /// recursion through self-referential types and multi-path arrival
    /// of the same root.
    pub fn contains_name(&self, name: &str) -> bool {
        self.schemas
            .iter()
            .any(|s| s.name().map(|n| n.as_str() == name).unwrap_or(false))
    }
}
