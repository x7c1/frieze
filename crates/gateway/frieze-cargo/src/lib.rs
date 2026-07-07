//! Cargo gateway for frieze.
//!
//! Implements the cargo-facing gateway traits of `frieze-usecase`:
//!
//! - [`CargoPackageResolver`] — resolves which workspace member a run
//!   targets via `cargo metadata`
//!   ([`frieze_usecase::PackageResolver`]).
//! - [`CargoSchemasCollector`] — generates a scratch crate that links
//!   the target crate, runs it through a cargo subprocess, and
//!   receives the canonical components dump the scratch binary writes
//!   to stdout ([`frieze_usecase::SchemasCollector`]).
//!
//! Everything cargo-related — workspace discovery, scratch-crate
//! generation, the subprocess invocation, stdout/stderr handling —
//! stays inside this crate. The filesystem gateway crate handles the
//! target package's own files instead, and the two know nothing about
//! each other. Only the composition root wires concrete gateways
//! together.

mod error;
pub use error::Error;

mod collector;
pub use collector::CargoSchemasCollector;

mod resolve;
pub use resolve::CargoPackageResolver;

mod inspect;
mod scratch;
