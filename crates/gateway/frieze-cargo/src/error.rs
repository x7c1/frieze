//! The internal error type of the cargo gateway.

use std::io;

use thiserror::Error;

/// Raw failures the cargo gateway can hit while collecting schemas.
///
/// This type stays internal to the gateway's own plumbing: at the
/// trait boundary the implementation maps what happened into
/// [`frieze_usecase::Error::SchemasCollect`] with the matching cause,
/// so callers of the use-case layer never see this type in a
/// signature.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O failure while preparing the scratch crate directory.
    #[error("scratch crate I/O failure: {0}")]
    ScratchIo(io::Error),
    /// Writing a scratch crate source file from its template failed.
    #[error("cannot write a scratch crate file: {0}")]
    ScratchTemplateWrite(io::Error),
    /// The output of `cargo metadata` (or the package shape it
    /// describes) cannot be interpreted — e.g. the target package is
    /// missing from it, or has no lib target the scratch crate could
    /// link.
    #[error("cannot interpret the package layout: {message}")]
    PackageInspect { message: String },
    /// The target crate declares its frieze dependency with the
    /// `inventory` feature disabled, so its derive output submits no
    /// schema registrations.
    #[error("the target crate disables the frieze `inventory` feature")]
    InventoryDisabled,
    /// The target crate's declared `frieze` version requirement can
    /// never match the exact frieze version the scratch crate pins
    /// (this crate's own version), so cargo would resolve two frieze
    /// instances and the collection would silently see no schemas.
    #[error(
        "the declared frieze requirement `{requirement}` cannot match \
         the CLI's frieze version {cli_version}"
    )]
    FriezeVersionMismatch {
        requirement: String,
        cli_version: String,
    },
    /// The cargo subprocess failed (spawn failure surfaces as
    /// `exit_code: None`). `stderr` carries any additionally captured
    /// output and is empty when everything was already streamed to the
    /// terminal.
    #[error(
        "cargo invocation failed{}",
        render_invocation_failure(exit_code, stderr)
    )]
    CargoInvocation {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The scratch binary's stdout was not a valid canonical
    /// components dump.
    #[error("cannot parse the scratch binary's stdout: {0}")]
    StdoutParse(serde_json::Error),
}

/// Renders the detail suffix of a failed cargo invocation: the exit
/// code when one exists and any captured stderr.
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
