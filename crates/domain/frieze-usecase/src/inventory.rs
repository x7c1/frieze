//! Submission channel and iterator for the `inventory`-based
//! auto-collection layer.
//!
//! `#[derive(Schema)]` emits a `SchemaRoot` entry per non-generic type
//! through the facade's `__private::inventory_submit!` wrapper macro.
//! The `inventory` feature is on by default on the facade (and therefore
//! on this crate), so every such entry lands in the `inventory` linker
//! section and [`SchemasBuilder::from_inventory`] walks them all,
//! invoking each entry's `register_fn` to drive the transitive
//! [`crate::Register::register_into`] walk rooted at the submitted type.
//!
//! Generic types (`Page<T>`) cannot be inventory entries — Rust's
//! `static` cannot hold generic types, so the derive emits no submission
//! for them. They are still auto-collected transitively when a
//! non-generic root's field references the concrete instantiation
//! (`struct Foo { page: Page<Bar> }`): the derived `register_into` walks
//! into `<Page<Bar> as Register>::register_into` at runtime.
//!
//! This module is compiled only under `#[cfg(feature = "inventory")]`,
//! which remains a gate so consumers that opt out (via
//! `default-features = false`) pay no link-time or runtime cost for the
//! channel.

use crate::schemas_builder::SchemasBuilder;

/// Submission entry for the `inventory`-based collection channel.
///
/// `#[derive(Schema)]` emits one of these per non-generic input via the
/// facade's `inventory_submit!` wrapper, capturing the schema's
/// registration name (for debug / collision diagnostics) and a function
/// pointer to the derived `register_into` so the transitive walk runs
/// at iteration time.
pub struct SchemaRoot {
    /// The schema's registration name as known at derive time
    /// (`<T as Schema>::name()` for a non-generic `T`).
    pub name: &'static str,
    /// Function pointer to `<T as Register>::register_into`.
    /// `SchemasBuilder::from_inventory` invokes this on every iterated
    /// entry to drive the transitive registration walk.
    pub register_fn: fn(&mut SchemasBuilder),
}

inventory::collect!(SchemaRoot);

/// Submission entry that declares a Rust `mod` to be a namespace for
/// OAS schema-name composition.
///
/// `#[frieze(namespace)]` on a `mod` declaration emits one of these via
/// the facade's `inventory_namespace!` wrapper, capturing the parent
/// module path (the `module_path!()` at the attribute site) and the
/// mod's local ident (`stringify!(v1)`). The full namespace path is
/// reconstructed as `format!("{}::{}", parent_path, local_name)` and
/// stored in a process-wide set consulted by
/// [`crate::naming::compose_schema_name`].
///
/// The attribute macro never touches the mod's contents; this side
/// channel records only the fact "this mod is a namespace" so derive
/// output can later walk `module_path!()` and decide which segments to
/// keep.
pub struct Namespace {
    /// `module_path!()` at the position where `#[frieze(namespace)]`
    /// is applied — the parent module of the declared namespace.
    pub parent_path: &'static str,
    /// The mod's local identifier (`stringify!(v1)` for
    /// `#[frieze(namespace)] pub mod v1;`).
    pub local_name: &'static str,
}

inventory::collect!(Namespace);

impl SchemasBuilder {
    /// Iterates every `inventory`-submitted [`SchemaRoot`] and invokes
    /// the entry's `register_fn` on `self`, producing the transitive
    /// closure rooted at every `#[derive(Schema)]` non-generic type
    /// linked into the current binary.
    ///
    /// `from_inventory()` is composable with [`SchemasBuilder::add`]:
    /// callers can extend the inventory-derived set with explicit roots
    /// such as `Box<i64>` that cannot themselves live in an inventory
    /// entry (Rust's `static` cannot hold the generic blanket impl
    /// instance).
    ///
    /// # Test independence
    ///
    /// `inventory` aggregates per binary, so every test in a given test
    /// binary observes the same submission set. Tests that need an
    /// isolated schemas set should reach for [`SchemasBuilder::add`]
    /// instead.
    pub fn from_inventory(mut self) -> Self {
        for entry in inventory::iter::<SchemaRoot>() {
            (entry.register_fn)(&mut self);
        }
        self
    }
}
