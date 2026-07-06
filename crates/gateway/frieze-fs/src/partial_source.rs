//! The filesystem-backed [`PartialSource`] implementation.

use frieze_model::PartialFilePath;
use frieze_openapi::Document;
use frieze_usecase::{PartialSource, Result};

/// Loads a partial OAS document from disk, parsing YAML or JSON
/// according to the path's format.
#[derive(Debug, Default)]
pub struct FsPartialSource;

impl FsPartialSource {
    pub fn new() -> Self {
        Self
    }
}

impl PartialSource for FsPartialSource {
    /// Not implemented yet: the file reading and parsing logic lands
    /// together with the CLI that drives it.
    fn load(&self, _path: &PartialFilePath) -> Result<Document> {
        todo!("implemented together with the CLI generate flow")
    }
}
