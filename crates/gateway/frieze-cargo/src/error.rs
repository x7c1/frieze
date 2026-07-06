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
    /// The cargo subprocess failed (spawn failure surfaces as
    /// `exit_code: None`).
    #[error("cargo invocation failed (exit code {exit_code:?}): {stderr}")]
    CargoInvocation {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The scratch binary's stdout was not a valid canonical
    /// components dump.
    #[error("cannot parse the scratch binary's stdout: {0}")]
    StdoutParse(serde_json::Error),
}
