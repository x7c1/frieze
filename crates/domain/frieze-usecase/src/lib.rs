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

#[cfg(all(feature = "oas-3-0", feature = "oas-3-1"))]
compile_error!(
    "frieze: features `oas-3-0` and `oas-3-1` are mutually exclusive; pick exactly one."
);

#[cfg(not(any(feature = "oas-3-0", feature = "oas-3-1")))]
compile_error!(
    "frieze: one of features `oas-3-0` or `oas-3-1` must be enabled (default is `oas-3-0`)."
);

mod boundary;
pub use boundary::components_from_schemas;

mod compose;
pub use compose::{compose, from_schemas};
