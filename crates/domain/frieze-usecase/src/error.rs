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
//!   [`Error::SchemasCollect`] / [`Error::OutputWrite`] /
//!   [`Error::OutputCheck`]) — named after
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

use frieze_model::{
    ConfigError, OasVersionCheck, OutputFilePath, OutputName, PackageName, PackageRoot,
    PartialFilePath,
};
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
    /// A partial document declares an OAS version outside the
    /// major.minor line the package metadata pins via its optional
    /// `oas-version` consistency check.
    ///
    /// The check never selects a version — the generated document
    /// always follows the partial's `openapi:` field — it only rejects
    /// a partial that contradicts the declared expectation.
    #[error(
        "output `{output}`: the partial document `{partial}` declares \
         `openapi: {partial_version}`, but the package metadata pins \
         `oas-version = \"{expected}\"`: update the partial's `openapi:` \
         field or the pinned `oas-version` so they agree"
    )]
    OasVersionMismatch {
        /// The output whose partial failed the check.
        output: OutputName,
        /// The partial document that was checked.
        partial: PartialFilePath,
        /// The verbatim `openapi:` field of the partial document.
        partial_version: String,
        /// The major.minor line the metadata pins.
        expected: OasVersionCheck,
    },
    /// The package's `Cargo.toml` has no `[package.metadata.frieze]`
    /// section.
    #[error(
        "no `[package.metadata.frieze]` section in the Cargo.toml of \
         `{root}`: declare at least one \
         `[[package.metadata.frieze.outputs]]` entry"
    )]
    MissingFriezeSection { root: PackageRoot },
    /// The package declares a `[package.metadata.frieze]` section but
    /// no outputs; at least one output is required.
    #[error(
        "no outputs defined in the Cargo.toml of `{root}`: declare at \
         least one `[[package.metadata.frieze.outputs]]` entry"
    )]
    NoOutputsDefined { root: PackageRoot },
    /// Resolving which package the run targets failed.
    #[error("cannot resolve the target package: {cause}")]
    PackageResolve { cause: PackageResolveCause },
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
    /// Comparing an existing output file against the generated
    /// document (check mode) could not be carried out. A file that is
    /// merely stale or missing is *not* this error — that is a
    /// [`crate::gateway::CheckOutcome`] verdict in the report.
    #[error("failed to check the output `{path}`: {cause}")]
    OutputCheck {
        path: OutputFilePath,
        cause: OutputCheckCause,
    },
}

/// Machine-readable detail of an [`Error::PackageResolve`] failure.
#[derive(Debug, Error)]
pub enum PackageResolveCause {
    /// The current directory the resolution starts from cannot be
    /// determined.
    #[error("cannot determine the current directory: {0}")]
    CurrentDir(io::Error),
    /// The `cargo metadata` invocation that discovers the enclosing
    /// workspace could not run or exited nonzero — e.g. the current
    /// directory is not inside a cargo project. Cargo's own stderr is
    /// relayed verbatim.
    #[error(
        "cargo metadata failed{}",
        render_invocation_failure(exit_code, stderr)
    )]
    CargoMetadata {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The `cargo metadata` output could not be interpreted.
    #[error("cannot interpret the cargo metadata output: {message}")]
    MetadataParse { message: String },
    /// The explicitly requested package (`-p <name>`) is not a member
    /// of the enclosing workspace. `available` lists the members.
    #[error(
        "package `{requested}` is not a member of this workspace \
         (members: {})",
        render_members(available)
    )]
    RequestedPackageNotFound {
        requested: PackageName,
        available: Vec<PackageName>,
    },
    /// The `[workspace.metadata.frieze] package` declaration names a
    /// package that is not a member of the workspace.
    #[error(
        "`[workspace.metadata.frieze]` declares `package = \
         \"{requested}\"`, which is not a member of this workspace \
         (members: {})",
        render_members(available)
    )]
    DefaultPackageNotFound {
        requested: PackageName,
        available: Vec<PackageName>,
    },
    /// Nothing selects a target package: the workspace has several
    /// members, the invocation names none, and the workspace declares
    /// no default.
    #[error(
        "no target package selected: pass `-p <name>` or declare \
         `package = \"...\"` under `[workspace.metadata.frieze]` in \
         the workspace root Cargo.toml (members: {})",
        render_members(available)
    )]
    NoTargetPackage { available: Vec<PackageName> },
    /// The `[workspace.metadata.frieze]` table contains a key the
    /// schema does not define. Unknown keys are rejected rather than
    /// silently ignored; when the key is a near-miss of a known one,
    /// the message suggests it.
    #[error(
        "unknown key `{key}` in `[workspace.metadata.frieze]`{}",
        render_suggestion(suggestion)
    )]
    UnknownKey {
        key: String,
        /// The closest known key of the table, when one is within a
        /// small edit distance of `key`.
        suggestion: Option<String>,
    },
    /// A key in the `[workspace.metadata.frieze]` table holds a value
    /// of the wrong TOML type.
    #[error("key `{key}` in `[workspace.metadata.frieze]` must be {expected}")]
    UnexpectedType { key: String, expected: &'static str },
}

/// Renders a member list for the resolve failures, in the stable
/// (sorted) order the causes carry.
fn render_members(members: &[PackageName]) -> String {
    members
        .iter()
        .map(PackageName::as_str)
        .collect::<Vec<_>>()
        .join(", ")
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
    #[error("Cargo.toml has no `[package]` table")]
    MissingPackageTable,
    /// A table under `[package.metadata.frieze]` contains a key the
    /// schema does not define. Unknown keys are rejected rather than
    /// silently ignored; when the key is a near-miss of a known one,
    /// the message suggests it.
    #[error("unknown key `{key}` in `{table}`{}", render_suggestion(suggestion))]
    UnknownKey {
        key: String,
        table: String,
        /// The closest known key of the table, when one is within a
        /// small edit distance of `key`.
        suggestion: Option<String>,
    },
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
    #[error(
        "cargo invocation failed{}",
        render_invocation_failure(exit_code, stderr)
    )]
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
         so its schemas cannot be collected: re-enable it by removing \
         `default-features = false` from the frieze dependency, or by \
         adding \"inventory\" to its `features` list"
    )]
    InventoryDisabled,
    /// The target crate's declared `frieze` version requirement cannot
    /// match the frieze version the collector pins, so the two would
    /// resolve as separate instances and no schemas could be
    /// collected.
    #[error(
        "the target crate requires `frieze = \"{requirement}\"`, which \
         does not match the frieze version this cargo-frieze collects \
         with ({cli_version}): align the crate's frieze dependency with \
         the installed CLI, or install the matching CLI with \
         `cargo install frieze-cli --version {cli_version}`"
    )]
    FriezeVersionMismatch {
        /// The declared requirement, as cargo normalizes it.
        requirement: String,
        /// The exact frieze version this CLI release collects with.
        cli_version: String,
    },
}

/// Renders the `did you mean` suffix of an unknown-key message, or
/// nothing when no known key is close enough to suggest.
fn render_suggestion(suggestion: &Option<String>) -> String {
    match suggestion {
        Some(key) => format!(" (did you mean `{key}`?)"),
        None => String::new(),
    }
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
    /// Serializing the document to JSON failed. (YAML rendering is
    /// infallible, so it has no counterpart here.)
    #[error("JSON serialize error: {0}")]
    SerializeJson(serde_json::Error),
}

/// Machine-readable detail of an [`Error::OutputCheck`] failure.
#[derive(Debug, Error)]
pub enum OutputCheckCause {
    /// The process lacks permission to read the existing output file.
    #[error("permission denied")]
    PermissionDenied,
    /// Any other I/O failure while reading the existing output file.
    #[error("{0}")]
    Read(io::Error),
    /// Serializing the document to JSON for the comparison failed.
    /// (YAML rendering is infallible, so it has no counterpart here.)
    #[error("JSON serialize error: {0}")]
    SerializeJson(serde_json::Error),
}
