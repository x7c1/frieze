//! The internal error type of the filesystem gateway.

use std::io;

use thiserror::Error;

/// Raw failures the filesystem gateway can hit while doing its work.
///
/// This type stays internal to the gateway's own plumbing: at each
/// trait boundary the implementation maps what happened into the
/// semantic boundary variants of [`frieze_usecase::Error`]
/// (`MetadataRead` / `PartialRead` / `OutputWrite`) with the matching
/// cause, so callers of the use-case layer never see this type in a
/// signature.
///
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O failure (read, write, create-dir, ...).
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A YAML parse or serialize failure.
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    /// A JSON parse or serialize failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A TOML parse failure (manifest reading).
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}
