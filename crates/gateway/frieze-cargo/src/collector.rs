//! The cargo-backed [`SchemasCollector`] implementation.

use frieze_model::PackageMetadata;
use frieze_openapi::Components;
use frieze_usecase::{Result, SchemasCollector};

/// Collects the schemas the target crate registers by generating a
/// scratch crate that links it, running the scratch binary via cargo,
/// and parsing the canonical components dump from its stdout.
#[derive(Debug, Default)]
pub struct CargoSchemasCollector;

impl CargoSchemasCollector {
    pub fn new() -> Self {
        Self
    }
}

impl SchemasCollector for CargoSchemasCollector {
    /// Not implemented yet: the scratch-crate generation and cargo
    /// subprocess logic lands together with the CLI that drives it.
    fn collect(&self, _metadata: &PackageMetadata) -> Result<Components> {
        todo!("implemented together with the CLI generate flow")
    }
}
