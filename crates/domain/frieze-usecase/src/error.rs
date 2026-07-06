//! The error type of the use-case layer.
//!
//! Failures are layered the same way the crates are:
//!
//! - Lower-layer construction failures ([`frieze_model::Error`] /
//!   [`frieze_model::ConfigError`]) are wrapped verbatim in the
//!   [`Error::Model`] / [`Error::Config`] variants.
//! - Use-case-specific failures (unknown output name, missing or empty
//!   configuration) are dedicated variants.
//! - Failures at a gateway boundary are grouped by *semantic category*
//!   ([`Error::MetadataRead`] / [`Error::PartialRead`] /
//!   [`Error::SchemasCollect`] / [`Error::OutputWrite`]) — named after
//!   what was being attempted, never after a concrete gateway type —
//!   with a machine-readable `Cause` sub-enum carrying the detail.
//!   Gateway implementations map their internal errors into these
//!   variants at the trait boundary.
//!
//! Callers can pattern-match in two steps: first on the semantic
//! category, then on the cause. None of the enums here are
//! `#[non_exhaustive]`: adding a variant should surface a compile
//! error at every match site so no handler silently ignores a new
//! failure mode. They also deliberately do not derive `PartialEq`,
//! since several variants carry dynamic values (`std::io::Error`,
//! serde errors) without a stable notion of equality.

use std::io;

use frieze_model::{ConfigError, OutputFilePath, OutputName, PackageRoot, PartialFilePath};
use thiserror::Error;

/// The `Result` alias used across the use-case layer, including the
/// gateway traits.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors surfaced by the use-case layer.
#[derive(Debug, Error)]
pub enum Error {
    /// A schema-domain failure from `frieze-model` (schema validation,
    /// composition preconditions such as a partial document that
    /// already contains schemas, ...).
    #[error(transparent)]
    Model(#[from] frieze_model::Error),
    /// A generation-configuration construction failure from
    /// `frieze-model` (invalid path, name, format, or a collision
    /// inside the package metadata).
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The caller asked for an output name the package does not
    /// declare. `available` lists the names that exist.
    #[error(
        "output `{requested}` is not defined (available: {})",
        available.iter().map(OutputName::as_str).collect::<Vec<_>>().join(", ")
    )]
    UnknownOutputName {
        requested: OutputName,
        available: Vec<OutputName>,
    },
    /// The package's `Cargo.toml` has no `[package.metadata.frieze]`
    /// section.
    #[error("no `[package.metadata.frieze]` section in `{root}`")]
    MissingFriezeSection { root: PackageRoot },
    /// The package declares a `[package.metadata.frieze]` section but
    /// no outputs; at least one output is required.
    #[error(
        "no outputs defined in `{root}`: declare at least one \
         `[[package.metadata.frieze.outputs]]` entry"
    )]
    NoOutputsDefined { root: PackageRoot },
    /// Reading the package's generation configuration failed.
    #[error("failed to read the frieze metadata of `{root}`: {cause}")]
    MetadataRead {
        root: PackageRoot,
        cause: MetadataReadCause,
    },
    /// Reading or parsing a partial OAS document failed.
    #[error("failed to read the partial OAS document `{path}`: {cause}")]
    PartialRead {
        path: PartialFilePath,
        cause: PartialReadCause,
    },
    /// Collecting the schemas registered by the target crate failed.
    #[error("failed to collect schemas from the target crate: {cause}")]
    SchemasCollect { cause: SchemasCollectCause },
    /// Serializing or writing a generated document failed.
    #[error("failed to write the output `{path}`: {cause}")]
    OutputWrite {
        path: OutputFilePath,
        cause: OutputWriteCause,
    },
}

/// Machine-readable detail of an [`Error::MetadataRead`] failure.
#[derive(Debug, Error)]
pub enum MetadataReadCause {
    /// Reading the `Cargo.toml` file failed.
    #[error("cannot read Cargo.toml: {0}")]
    CargoManifestRead(io::Error),
    /// The `Cargo.toml` file is not valid TOML. The message is the
    /// parser's rendering of the failure; the concrete TOML parser is
    /// an implementation detail of the gateway, so no parser error
    /// type appears here.
    #[error("cannot parse Cargo.toml: {message}")]
    CargoManifestParse { message: String },
    /// The `Cargo.toml` has no `[package]` table.
    #[error("Cargo.toml has no [package] table")]
    MissingPackageTable,
    /// A table under `[package.metadata.frieze]` contains a key the
    /// schema does not define. Unknown keys are rejected rather than
    /// silently ignored.
    #[error("unknown key `{key}` in `{table}`")]
    UnknownKey { key: String, table: String },
    /// A required key is absent from one of the frieze metadata
    /// tables.
    #[error("missing required key `{key}` in `{table}`")]
    MissingKey { key: String, table: String },
    /// A key in one of the frieze metadata tables holds a value of the
    /// wrong TOML type (e.g. an integer where a string is required).
    #[error("key `{key}` in `{table}` must be {expected}")]
    UnexpectedType {
        key: String,
        table: String,
        expected: &'static str,
    },
}

/// Machine-readable detail of an [`Error::PartialRead`] failure.
#[derive(Debug, Error)]
pub enum PartialReadCause {
    /// The file vanished after the path was validated.
    #[error("file not found")]
    NotFound,
    /// The process lacks permission to read the file.
    #[error("permission denied")]
    PermissionDenied,
    /// Any other I/O failure while reading the file.
    #[error("{0}")]
    Io(io::Error),
    /// The file is not a valid YAML OAS document.
    #[error("YAML parse error: {0}")]
    YamlParse(serde_yaml::Error),
    /// The file is not a valid JSON OAS document.
    #[error("JSON parse error: {0}")]
    JsonParse(serde_json::Error),
}

/// Machine-readable detail of an [`Error::SchemasCollect`] failure.
#[derive(Debug, Error)]
pub enum SchemasCollectCause {
    /// Generating the scratch crate that links the target crate
    /// failed.
    #[error("cannot generate the scratch crate: {0}")]
    ScratchGenerate(io::Error),
    /// Inspecting the target package with `cargo metadata` produced
    /// output the collector cannot interpret (or the package shape is
    /// unusable, e.g. no lib target to link).
    #[error("cannot interpret the package layout: {message}")]
    PackageInspect { message: String },
    /// The cargo invocation that builds and runs the scratch crate
    /// failed. The build log itself goes to the user's terminal via
    /// stderr; `stderr` carries any additionally captured output and
    /// is empty when everything was already streamed through.
    #[error("cargo invocation failed{}", render_invocation_failure(exit_code, stderr))]
    CargoInvocation {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The scratch crate's stdout was not a valid canonical components
    /// dump.
    #[error("cannot parse the collected schemas: {0}")]
    ScratchStdoutParse(serde_json::Error),
    /// The target crate compiles with the `inventory` feature of
    /// frieze disabled, so no schemas can be collected from it.
    #[error(
        "the target crate disables the frieze `inventory` feature, \
         so its schemas cannot be collected"
    )]
    InventoryDisabled,
}

/// Renders the detail suffix of a failed cargo invocation: the exit
/// code when one exists (a spawn failure has none) and any captured
/// stderr. When stderr was streamed straight to the terminal nothing
/// is repeated here.
fn render_invocation_failure(exit_code: &Option<i32>, stderr: &str) -> String {
    let mut detail = match exit_code {
        Some(code) => format!(" (exit code {code})"),
        None => String::new(),
    };
    if !stderr.trim().is_empty() {
        detail.push_str(": ");
        detail.push_str(stderr.trim_end());
    }
    detail
}

/// Machine-readable detail of an [`Error::OutputWrite`] failure.
#[derive(Debug, Error)]
pub enum OutputWriteCause {
    /// The output's parent directory vanished and re-creating it
    /// failed.
    #[error("cannot create the parent directory: {0}")]
    ParentDirCreate(io::Error),
    /// Writing the file failed.
    #[error("{0}")]
    Write(io::Error),
    /// The process lacks permission to write the file.
    #[error("permission denied")]
    PermissionDenied,
    /// Serializing the document to YAML failed.
    #[error("YAML serialize error: {0}")]
    SerializeYaml(serde_yaml::Error),
    /// Serializing the document to JSON failed.
    #[error("JSON serialize error: {0}")]
    SerializeJson(serde_json::Error),
}
