//! The filesystem-backed [`MetadataSource`] implementation.

use frieze_model::{PackageMetadata, PackageRoot};
use frieze_usecase::{MetadataSource, Result};

/// Reads a package's generation configuration from the
/// `[package.metadata.frieze]` section of its `Cargo.toml`.
#[derive(Debug, Default)]
pub struct FsMetadataSource;

impl FsMetadataSource {
    pub fn new() -> Self {
        Self
    }
}

impl MetadataSource for FsMetadataSource {
    /// Not implemented yet: the manifest reading and parsing logic
    /// lands together with the CLI that drives it.
    fn read(&self, _root: &PackageRoot) -> Result<PackageMetadata> {
        todo!("implemented together with the CLI generate flow")
    }
}
