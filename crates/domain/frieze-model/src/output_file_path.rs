//! A validated path for a generated OAS document.

use std::path::{Path, PathBuf};

use crate::config_error::ConfigError;
use crate::output_format::OutputFormat;
use crate::partial_file_path::format_from_path;

/// A path that a generated OAS document can be written to.
///
/// Constructed via [`OutputFilePath::try_from_path`]; the fields are
/// private so the invariants cannot be bypassed.
///
/// # Invariants
///
/// - the file extension is one of the supported set
///   ([`OutputFormat::ALLOWED_EXTENSIONS`]). The extension is the
///   user's declaration of the output format; the corresponding
///   [`OutputFormat`] is lifted at construction and exposed via
///   [`Self::format`]
/// - the parent directory exists — the constructor creates it
///   (`create_dir_all`, idempotent) if needed, so a validated value is
///   known to be writable-into at construction time
///
/// The file itself does not need to exist: it is created or
/// overwritten when the document is persisted. The path is stored
/// verbatim (not canonicalized) so error messages and logs show the
/// spelling the user configured.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputFilePath {
    path: PathBuf,
    format: OutputFormat,
}

impl OutputFilePath {
    /// Validates the extension, ensures the parent directory exists
    /// (creating it if needed), and stores the path.
    pub fn try_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let format = format_from_path(path)?;
        match path.parent() {
            // A bare file name (or a root path) has no parent directory
            // to create; writes resolve against the current directory.
            None => {}
            Some(parent) if parent.as_os_str().is_empty() => {}
            Some(parent) => {
                std::fs::create_dir_all(parent).map_err(|cause| {
                    ConfigError::OutputParentUnwritable {
                        got: path.to_path_buf(),
                        cause,
                    }
                })?;
            }
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

impl std::fmt::Display for OutputFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.display().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_new_file_in_an_existing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openapi.yaml");
        let output = OutputFilePath::try_from_path(&path).unwrap();
        assert_eq!(output.as_path(), path);
        assert_eq!(output.format(), OutputFormat::Yaml);
    }

    #[test]
    fn creates_a_missing_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/deeper/openapi.json");
        let output = OutputFilePath::try_from_path(&path).unwrap();
        assert_eq!(output.format(), OutputFormat::Json);
        assert!(path.parent().unwrap().is_dir());
    }

    #[test]
    fn rejects_an_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        for file_name in ["openapi.txt", "openapi"] {
            let path = dir.path().join(file_name);
            let result = OutputFilePath::try_from_path(&path);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::UnsupportedFileExtension { got, .. }) if *got == path
                ),
                "expected `{file_name}` to be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn rejects_an_uncreatable_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        // A regular file where a directory component is required makes
        // `create_dir_all` fail.
        let obstacle = dir.path().join("occupied");
        std::fs::write(&obstacle, "").unwrap();
        let path = obstacle.join("openapi.yaml");
        let result = OutputFilePath::try_from_path(&path);
        assert!(
            matches!(result, Err(ConfigError::OutputParentUnwritable { .. })),
            "expected the uncreatable parent to be rejected, got {result:?}"
        );
    }
}
