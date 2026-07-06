//! The error type for the generation-configuration domain types.
//!
//! [`ConfigError`] covers construction failures of the parsed
//! configuration types (package root / partial and output paths /
//! output names / package metadata). It is separate from
//! [`crate::Error`] because several variants carry dynamic values such
//! as [`std::io::Error`], which rules out the `PartialEq` derive that
//! `crate::Error` exposes for the schema-domain tests.
//!
//! The enum is deliberately **not** `#[non_exhaustive]`: within this
//! workspace, adding a variant should surface a compile error at every
//! match site so no handler silently ignores a new failure mode.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::output_name::OutputName;

/// Errors raised by the smart constructors of the generation-
/// configuration types.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The path handed to [`crate::PackageRoot::try_from_path`] does
    /// not point to an existing directory.
    #[error("package root `{}` is not a directory", got.display())]
    PackageRootNotDirectory { got: PathBuf },
    /// The directory handed to [`crate::PackageRoot::try_from_path`]
    /// contains no `Cargo.toml`.
    #[error("no Cargo.toml found in `{}`", root.display())]
    MissingCargoToml { root: PathBuf },
    /// Canonicalizing the package-root path failed (e.g. a component
    /// vanished between the directory check and canonicalization, or
    /// the process lacks permission to traverse it).
    #[error("cannot canonicalize package root `{}`: {cause}", got.display())]
    PackageRootCanonicalize { got: PathBuf, cause: io::Error },
    /// The path handed to [`crate::PartialFilePath::try_from_path`]
    /// does not point to an existing file.
    #[error("partial OAS document `{}` does not point to an existing file", got.display())]
    PartialFileNotFound { got: PathBuf },
    /// The parent directory of an output path could not be created.
    #[error("cannot create the parent directory of output path `{}`: {cause}", got.display())]
    OutputParentUnwritable { got: PathBuf, cause: io::Error },
    /// A file path carries no extension, or one outside the supported
    /// set. `allowed` lists the accepted extensions.
    #[error(
        "unsupported file extension on `{}` (expected one of: {})",
        got.display(),
        allowed.join(", ")
    )]
    UnsupportedFileExtension {
        got: PathBuf,
        allowed: &'static [&'static str],
    },
    /// An extension string handed to
    /// [`crate::OutputFormat::try_from_extension`] is outside the
    /// supported set.
    #[error("unsupported format extension `{got}` (expected one of: {})", allowed.join(", "))]
    UnsupportedFormatExtension {
        got: String,
        allowed: &'static [&'static str],
    },
    /// An output name is empty.
    #[error("output name must not be empty")]
    OutputNameEmpty,
    /// An output name contains a character outside `[a-z0-9_-]`.
    /// `at` is the byte offset of the offending character `ch`.
    #[error(
        "output name `{got}` contains invalid character `{ch}` at byte {at} \
         (allowed characters: a-z, 0-9, `_`, `-`)"
    )]
    OutputNameInvalidChar { got: String, at: usize, ch: char },
    /// A string is not a valid cargo package name.
    #[error(
        "`{got}` is not a valid cargo package name \
         (non-empty, characters a-z, A-Z, 0-9, `_`, `-`, must not start with a digit)"
    )]
    PackageNameInvalid { got: String },
    /// A string is not a valid cargo feature name.
    #[error(
        "`{got}` is not a valid cargo feature name \
         (starts with an ASCII alphanumeric or `_`, followed by \
         ASCII alphanumerics, `_`, `-`, `.`, or `+`)"
    )]
    CargoFeatureNameInvalid { got: String },
    /// A declared OAS version-check value is outside the supported set
    /// (`"3.0"` / `"3.1"`).
    #[error("invalid OAS version check value `{got}` (expected \"3.0\" or \"3.1\")")]
    OasVersionCheckInvalid { got: String },
    /// Two outputs in the same package declare the same name.
    #[error("output name `{name}` is declared more than once")]
    OutputNameCollision { name: OutputName },
    /// Two or more outputs in the same package write to the same path.
    /// `used_by` lists every output name that targets the path.
    #[error(
        "output path `{}` is used by more than one output: {}",
        path.display(),
        used_by.iter().map(OutputName::as_str).collect::<Vec<_>>().join(", ")
    )]
    OutputPathCollision {
        path: PathBuf,
        used_by: Vec<OutputName>,
    },
}
