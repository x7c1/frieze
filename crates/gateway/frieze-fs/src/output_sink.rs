//! The filesystem-backed [`OutputSink`] implementation.

use frieze_model::{OutputFilePath, OutputFormat};
use frieze_openapi::Document;
use frieze_usecase::{OutputSink, Result};

/// Serializes a generated document (YAML or JSON, per output) and
/// writes it to its output path.
#[derive(Debug, Default)]
pub struct FsOutputSink;

impl FsOutputSink {
    pub fn new() -> Self {
        Self
    }
}

impl OutputSink for FsOutputSink {
    /// Not implemented yet: the serialize-and-write logic lands
    /// together with the CLI that drives it.
    fn persist(
        &self,
        _target: &OutputFilePath,
        _document: &Document,
        _format: OutputFormat,
    ) -> Result<()> {
        todo!("implemented together with the CLI generate flow")
    }
}
