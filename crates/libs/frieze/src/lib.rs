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
//! called; the derived `Register::register_into` walks each root's
//! transitive dependency graph (including generic field instantiations
//! such as `Page<Bar>`) automatically.
//!
//! Consumers that want to skip the linker section entirely can opt out
//! with `default-features = false` (selecting an OAS version feature
//! such as `oas-3-0` explicitly). With the feature off, the derive's
//! submission site expands to a no-op so feature-gated code paths in
//! user crates stay valid in both configurations.

pub use frieze_macros::{frieze, Schema};
pub use frieze_model::{
    Error, Maybe, Presence, Property, PropertyName, PropertyType, SchemaName, Schemas,
};
pub use frieze_openapi::{to_yaml, Components, Document, Info};
pub use frieze_usecase::{compose, from_schemas, Register, Schema, SchemasBuilder};

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
/// `::frieze::__private::inventory_submit! { "TypeName", <TypeName as ::frieze::Register>::register_into }`
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

/// Wrapper macro used by `#[frieze(namespace)]` to submit a namespace
/// declaration to the `inventory` collection channel.
///
/// The attribute macro emits a call of the form
/// `::frieze::__private::inventory_namespace! { "<mod_ident>" }`
/// next to the original `mod` declaration. The macro routes the call
/// so that:
///
/// - With the `inventory` Cargo feature enabled, the call expands to
///   `inventory::submit!` of a `__private::Namespace` value, capturing
///   `module_path!()` at the attribute site as the namespace's
///   `parent_path` so the full path
///   `format!("{}::{}", parent_path, local_name)` can be reconstructed
///   later by [`frieze_usecase::compose_schema_name`].
/// - Without the feature, the call expands to nothing and the
///   attribute behaves as a transparent pass-through.
///
/// The indirection mirrors [`__frieze_inventory_submit`]: the
/// proc-macro crate stays feature-agnostic and the facade decides
/// whether the submission has runtime effect.
#[cfg(feature = "inventory")]
#[doc(hidden)]
#[macro_export]
macro_rules! __frieze_inventory_namespace {
    ($local_name:expr $(,)?) => {
        $crate::__private::inventory::submit! {
            $crate::__private::Namespace {
                parent_path: ::core::module_path!(),
                local_name: $local_name,
            }
        }
    };
}

/// No-op counterpart to [`__frieze_inventory_namespace`] for builds
/// with the `inventory` feature disabled. Symmetric matcher with the
/// `cfg(feature = "inventory")` arm so the attribute macro emits a
/// single, predictable invocation in either configuration.
#[cfg(not(feature = "inventory"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __frieze_inventory_namespace {
    ($local_name:expr $(,)?) => {};
}

/// Implementation details exposed only so the derive macro's expansion can
/// reach the underlying crates without users having to depend on them
/// directly. Not covered by semver.
#[doc(hidden)]
pub mod __private {
    pub use frieze_model;
    pub use frieze_openapi;
    pub use frieze_usecase;

    // Re-export the wrapper macros under the `__private` path so
    // derive / attribute output writes a single, predictable
    // invocation: `::frieze::__private::inventory_submit! { ... }` and
    // `::frieze::__private::inventory_namespace! { ... }`. The macros
    // themselves are `#[macro_export]`-ed at the crate root with
    // name-spaced identifiers to avoid shadowing anything in user code.
    pub use crate::__frieze_inventory_namespace as inventory_namespace;
    pub use crate::__frieze_inventory_submit as inventory_submit;

    // Helper used by `#[derive(Schema)]`-generated `Schema::name()`
    // bodies to fold `module_path!()` against the namespace set
    // populated by `#[frieze(namespace)]`.
    pub use frieze_usecase::compose_schema_name;

    // The `inventory` crate re-export is only available when the
    // feature is enabled; the wrapper macros above branch on the same
    // cfg so the path is only used when valid.
    #[cfg(feature = "inventory")]
    pub use ::inventory;

    // `Namespace` and `SchemaRoot` are only defined when the feature
    // is on, since their sole purpose is to be the value types for the
    // `inventory` linker sections. The macros above reference these
    // re-exports only in the feature-on arms.
    #[cfg(feature = "inventory")]
    pub use frieze_usecase::{Namespace, SchemaRoot};
}
