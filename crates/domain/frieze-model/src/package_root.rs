//! A validated package root directory.

use std::path::{Path, PathBuf};

use crate::config_error::ConfigError;

/// A directory that is guaranteed to contain a `Cargo.toml`, held as a
/// canonicalized absolute path.
///
/// Constructed via [`PackageRoot::try_from_path`]; the inner path is
/// private so the invariants cannot be bypassed.
///
/// # Invariants
///
/// - the path pointed to an existing directory at construction time
/// - that directory contained a `Cargo.toml` file
/// - the stored path is canonicalized (absolute, symlinks resolved),
///   so two roots reached through different spellings compare equal
///
/// # I/O
///
/// The constructor performs synchronous filesystem checks
/// (`is_dir` / `canonicalize` / `is_file`). Like every check against a
/// live filesystem, the guarantee is a snapshot: the directory can of
/// course be deleted after construction, and consumers surface such
/// late failures through their own I/O errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRoot(PathBuf);

impl PackageRoot {
    /// Validates that `path` is an existing directory containing a
    /// `Cargo.toml`, and stores its canonicalized form.
    pub fn try_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(ConfigError::PackageRootNotDirectory {
                got: path.to_path_buf(),
            });
        }
        let canonical =
            path.canonicalize()
                .map_err(|cause| ConfigError::PackageRootCanonicalize {
                    got: path.to_path_buf(),
                    cause,
                })?;
        if !canonical.join("Cargo.toml").is_file() {
            return Err(ConfigError::MissingCargoToml { root: canonical });
        }
        Ok(Self(canonical))
    }

    /// The canonicalized root directory.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// The path of the `Cargo.toml` inside this root.
    pub fn cargo_toml(&self) -> PathBuf {
        self.0.join("Cargo.toml")
    }
}

impl std::fmt::Display for PackageRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_directory_containing_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        let root = PackageRoot::try_from_path(dir.path()).unwrap();
        assert!(root.as_path().is_absolute());
        assert!(root.cargo_toml().is_file());
    }

    #[test]
    fn rejects_a_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-dir");
        let result = PackageRoot::try_from_path(&missing);
        assert!(
            matches!(
                &result,
                Err(ConfigError::PackageRootNotDirectory { got }) if *got == missing
            ),
            "expected the missing directory to be rejected, got {result:?}"
        );
    }

    #[test]
    fn rejects_a_file_as_root() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Cargo.toml");
        std::fs::write(&file, "[package]\n").unwrap();
        let result = PackageRoot::try_from_path(&file);
        assert!(
            matches!(result, Err(ConfigError::PackageRootNotDirectory { .. })),
            "expected a plain file to be rejected as root, got {result:?}"
        );
    }

    #[test]
    fn rejects_a_directory_without_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = PackageRoot::try_from_path(dir.path());
        assert!(
            matches!(result, Err(ConfigError::MissingCargoToml { .. })),
            "expected a directory without Cargo.toml to be rejected, got {result:?}"
        );
    }
}
