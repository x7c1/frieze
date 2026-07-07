//! Filesystem gateway for frieze.
//!
//! Implements the filesystem-facing gateway traits of `frieze-usecase`:
//!
//! - [`FsMetadataSource`] — reads a package's `Cargo.toml` and parses
//!   its `[package.metadata.frieze]` section
//!   ([`frieze_usecase::MetadataSource`]).
//! - [`FsPartialSource`] — loads and parses partial OAS documents
//!   ([`frieze_usecase::PartialSource`]).
//! - [`FsOutputSink`] — serializes and writes generated documents
//!   ([`frieze_usecase::OutputSink`]).
//!
//! This crate is only about the *user-facing* files of the target
//! package (its manifest, its partial documents, its outputs); running
//! cargo to collect schemas is the separate cargo gateway crate's job,
//! and the two know nothing about each other. Only the composition
//! root wires concrete gateways together.

mod edit_distance;

mod error;
pub use error::Error;

mod metadata_source;
pub use metadata_source::FsMetadataSource;

mod partial_source;
pub use partial_source::FsPartialSource;

mod output_sink;
pub use output_sink::FsOutputSink;
