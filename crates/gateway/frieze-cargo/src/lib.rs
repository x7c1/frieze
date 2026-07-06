//! Cargo gateway for frieze.
//!
//! Implements [`frieze_usecase::SchemasCollector`]: it generates a
//! scratch crate that links the target crate, runs it through a cargo
//! subprocess, and receives the canonical components dump the scratch
//! binary writes to stdout ([`CargoSchemasCollector`]).
//!
//! Everything cargo-related — scratch-crate generation, the
//! subprocess invocation, stdout/stderr handling — stays inside this
//! crate. The filesystem gateway crate handles the target package's
//! own files instead, and the two know nothing about each other. Only
//! the composition root wires concrete gateways together.

mod error;
pub use error::Error;

mod collector;
pub use collector::CargoSchemasCollector;

mod inspect;
mod scratch;
