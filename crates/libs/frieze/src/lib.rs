//! `frieze` — generate OpenAPI Schema Objects from Rust types via
//! `proc-macros`.
//!
//! This crate is the facade end users depend on. It re-exports the
//! [`Schema`] trait, the [`SchemasBuilder`] builder, the convenience
//! [`schemas`] entry point, and the `#[derive(Schema)]` macro so that
//! `use frieze::Schema;` brings both the trait and the derive into scope.
//!
//! # Auto-collection via `inventory`
//!
//! [`SchemasBuilder::from_inventory`] is available out of the box —
//! the `inventory` Cargo feature is on by default. Every non-generic
//! `#[derive(Schema)]` type is registered into the per-binary
//! `inventory` linker section and iterated when `from_inventory()` is
//! called; the derived `Schema::register_into` walks each root's
//! transitive dependency graph (including generic field instantiations
//! such as `Page<Bar>`) automatically.
//!
//! Consumers that want to skip the linker section entirely can opt out
//! with `default-features = false` (selecting an OAS version feature
//! such as `oas-3-0` explicitly). With the feature off, the derive's
//! submission site expands to a no-op so feature-gated code paths in
//! user crates stay valid in both configurations.

pub use frieze_macros::Schema;
pub use frieze_model::{
    Error, Maybe, Presence, Property, PropertyName, PropertyType, SchemaName, Schemas,
};
pub use frieze_usecase::{to_value, to_yaml, Schema, SchemasBuilder};

/// Convenience entry point: returns a fresh [`SchemasBuilder`].
///
/// Equivalent to `SchemasBuilder::new()`.
pub fn schemas() -> SchemasBuilder {
    SchemasBuilder::new()
}

/// Wrapper macro used by `#[derive(Schema)]` to submit a non-generic
/// schema root to the `inventory` collection channel.
///
/// The derive emits a call of the form
/// `::frieze::__private::inventory_submit! { "TypeName", <TypeName as ::frieze::Schema>::register_into }`
/// for every non-generic struct or enum input. The macro routes the
/// pair through the facade so that:
///
/// - With the `inventory` Cargo feature enabled, the call expands to
///   `inventory::submit!` of a `__private::SchemaRoot` value, landing
///   the entry in the per-binary `inventory` linker section.
/// - Without the feature, the call expands to nothing and the derive
///   output compiles into a regular `impl Schema` with no global
///   side-effect.
///
/// This indirection lets the proc-macro crate (`frieze-macros`) stay
/// feature-agnostic: it always emits the same token stream, and the
/// facade — which is the only crate that knows the consumer's feature
/// state — decides whether the submission has runtime effect.
#[cfg(feature = "inventory")]
#[doc(hidden)]
#[macro_export]
macro_rules! __frieze_inventory_submit {
    ($name:expr, $register_fn:expr $(,)?) => {
        $crate::__private::inventory::submit! {
            $crate::__private::SchemaRoot {
                name: $name,
                register_fn: $register_fn,
            }
        }
    };
}

/// No-op counterpart to [`__frieze_inventory_submit`] for builds with
/// the `inventory` feature disabled. The derive always emits the
/// submission tokens; this arm discards them so consumers that opt out
/// of the feature (via `default-features = false`) pay no link-time or
/// runtime cost.
#[cfg(not(feature = "inventory"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __frieze_inventory_submit {
    ($name:expr, $register_fn:expr $(,)?) => {};
}

/// Implementation details exposed only so the derive macro's expansion can
/// reach the underlying crates without users having to depend on them
/// directly. Not covered by semver.
#[doc(hidden)]
pub mod __private {
    pub use frieze_model;
    pub use frieze_openapi;
    pub use frieze_usecase;

    // Re-export the wrapper macro under the `__private` path so derive
    // output writes a single, predictable invocation:
    // `::frieze::__private::inventory_submit! { ... }`. The macro itself
    // is `#[macro_export]`-ed at the crate root with a name-spaced
    // identifier to avoid shadowing anything in user code.
    pub use crate::__frieze_inventory_submit as inventory_submit;

    // The `inventory` crate re-export is only available when the
    // feature is enabled; the wrapper macro above branches on the same
    // cfg so the path is only used when valid.
    #[cfg(feature = "inventory")]
    pub use ::inventory;

    // `SchemaRoot` itself is only defined when the feature is on, since
    // its sole purpose is to be the value type for the `inventory`
    // linker section. The macro above references this re-export only in
    // the feature-on arm.
    #[cfg(feature = "inventory")]
    pub use frieze_usecase::SchemaRoot;
}
