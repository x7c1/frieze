//! Use cases for frieze.
//!
//! Owns the boundary conversion from `frieze-model` to
//! `frieze-openapi` (in [`boundary`], surfaced as
//! [`components_from_schemas`]) and the composition entry points
//! ([`compose`], [`compose_components`], [`from_schemas`]) that
//! produce a complete [`frieze_openapi::Document`] ready for
//! serialization.
//!
//! The user-facing contract traits (`Schema` / `Register`) and the
//! `SchemasBuilder` registry live in the `frieze` crate; this crate
//! consumes the [`frieze_model::Schemas`] aggregate they produce.
//!
//! Everything here is OAS-version-neutral: the conversion records
//! intent as data, and the target OAS version (3.0 / 3.1) only takes
//! effect when the resulting document is serialized — dispatched at
//! runtime on `Document::oas_version` by `frieze-openapi`.
//!
//! This crate also defines the execution seam for the generate flow:
//! the [`gateway`] traits abstract every external interaction
//! (configuration reading, partial loading, schema collection, output
//! writing), and the [`GenerateOas`] interactor orchestrates the flow
//! against those traits. Concrete gateway implementations live in
//! separate gateway crates this crate knows nothing about; a
//! composition-root crate injects them.

mod boundary;
pub use boundary::components_from_schemas;

mod compose;
pub use compose::{compose, compose_components, from_schemas};

pub mod gateway;
pub use gateway::{MetadataSource, OutputSink, PartialSource, SchemasCollector};

mod generate;
pub use generate::{GenerateOas, GenerateOasParams, Report, WrittenOutput};

mod error;
pub use error::{
    Error, MetadataReadCause, OutputWriteCause, PartialReadCause, Result, SchemasCollectCause,
};
