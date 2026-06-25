//! `frieze` — generate OpenAPI Schema Objects from Rust types via
//! `proc-macros`.
//!
//! This crate is the facade end users depend on. It re-exports the
//! [`Schema`] trait, the [`SchemasBuilder`] builder, the convenience
//! [`schemas`] entry point, and the `#[derive(Schema)]` macro so that
//! `use frieze::Schema;` brings both the trait and the derive into scope.

pub use frieze_macros::Schema;
pub use frieze_model::{Error, Property, PropertyName, PropertyType, SchemaName, Schemas};
pub use frieze_usecase::{to_value, to_yaml, Schema, SchemasBuilder};

/// Convenience entry point: returns a fresh [`SchemasBuilder`].
///
/// Equivalent to `SchemasBuilder::new()`.
pub fn schemas() -> SchemasBuilder {
    SchemasBuilder::new()
}

/// Implementation details exposed only so the derive macro's expansion can
/// reach the underlying crates without users having to depend on them
/// directly. Not covered by semver.
#[doc(hidden)]
pub mod __private {
    pub use frieze_model;
    pub use frieze_openapi;
    pub use frieze_usecase;
}
