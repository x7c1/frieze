//! A validated path to a partial OAS document.

use std::path::{Path, PathBuf};

use crate::config_error::ConfigError;
use crate::output_format::OutputFormat;

/// A path that is guaranteed to point to an existing partial OAS
/// document with a supported file extension.
///
/// Constructed via [`PartialFilePath::try_from_path`]; the fields are
/// private so the invariants cannot be bypassed.
///
/// # Invariants
///
/// - the path pointed to an existing file at construction time
/// - the file extension is one of the supported set
///   ([`OutputFormat::ALLOWED_EXTENSIONS`]); the corresponding
///   [`OutputFormat`] is lifted at construction and exposed via
///   [`Self::format`]
///
/// The path is stored verbatim (not canonicalized) so error messages
/// and logs show the spelling the user configured.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartialFilePath {
    path: PathBuf,
    format: OutputFormat,
}

impl PartialFilePath {
    /// Validates that `path` points to an existing file with a
    /// supported extension.
    pub fn try_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let format = format_from_path(path)?;
        if !path.is_file() {
            return Err(ConfigError::PartialFileNotFound {
                got: path.to_path_buf(),
            });
        }
        Ok(Self {
            path: path.to_path_buf(),
            format,
        })
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// The serialization format lifted from the file extension.
    pub fn format(&self) -> OutputFormat {
        self.format
    }
}

impl std::fmt::Display for PartialFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.display().fmt(f)
    }
}

/// Lifts the [`OutputFormat`] from a path's extension, attributing the
/// failure to the full path.
pub(crate) fn format_from_path(path: &Path) -> Result<OutputFormat, ConfigError> {
    let unsupported = || ConfigError::UnsupportedFileExtension {
        got: path.to_path_buf(),
        allowed: OutputFormat::ALLOWED_EXTENSIONS,
    };
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(unsupported)?;
    OutputFormat::try_from_extension(ext).map_err(|_| unsupported())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_an_existing_yaml_file_and_lifts_the_format() {
        let dir = tempfile::tempdir().unwrap();
        for (file_name, expected) in [
            ("partial.yaml", OutputFormat::Yaml),
            ("partial.yml", OutputFormat::Yaml),
            ("partial.json", OutputFormat::Json),
        ] {
            let path = dir.path().join(file_name);
            std::fs::write(&path, "openapi: 3.0.3\n").unwrap();
            let partial = PartialFilePath::try_from_path(&path).unwrap();
            assert_eq!(partial.as_path(), path);
            assert_eq!(partial.format(), expected, "for `{file_name}`");
        }
    }

    #[test]
    fn rejects_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such.yaml");
        let result = PartialFilePath::try_from_path(&missing);
        assert!(
            matches!(
                &result,
                Err(ConfigError::PartialFileNotFound { got }) if *got == missing
            ),
            "expected the missing file to be rejected, got {result:?}"
        );
    }

    #[test]
    fn rejects_an_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        for file_name in ["partial.toml", "partial"] {
            let path = dir.path().join(file_name);
            std::fs::write(&path, "").unwrap();
            let result = PartialFilePath::try_from_path(&path);
            assert!(
                matches!(result, Err(ConfigError::UnsupportedFileExtension { .. })),
                "expected `{file_name}` to be rejected, got {result:?}"
            );
        }
    }
}
