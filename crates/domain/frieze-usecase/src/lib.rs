//! Use cases for frieze.
//!
//! Owns the boundary conversion from `frieze-model` to
//! `frieze-openapi` (in [`boundary`], surfaced as
//! [`components_from_schemas`]) and the composition entry points
//! ([`compose`], [`from_schemas`]) that produce a complete
//! [`frieze_openapi::Document`] ready for serialization.
//!
//! The user-facing contract traits (`Schema` / `Register`) and the
//! `SchemasBuilder` registry live in the `frieze` crate; this crate
//! consumes the [`frieze_model::Schemas`] aggregate they produce.
//!
//! Everything here is OAS-version-neutral: the conversion records
//! intent as data, and the target OAS version (3.0 / 3.1) only takes
//! effect when the resulting document is serialized — dispatched at
//! runtime on `Document::oas_version` by `frieze-openapi`.

mod boundary;
pub use boundary::components_from_schemas;

mod compose;
pub use compose::{compose, from_schemas};
