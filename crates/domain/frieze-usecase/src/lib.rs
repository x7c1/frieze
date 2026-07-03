//! Use cases for frieze.
//!
//! Owns the private boundary conversion from `frieze-model` to
//! `frieze-openapi` (in [`boundary`]) and the composition entry points
//! ([`compose`], [`from_schemas`]) that produce a complete
//! [`frieze_openapi::Document`] ready for serialization.
//!
//! The user-facing contract traits (`Schema` / `Register`) and the
//! `SchemasBuilder` registry live in the `frieze` crate; this crate
//! consumes the [`frieze_model::Schemas`] aggregate they produce.
//!
//! # Feature flags
//!
//! Exactly one of `oas-3-0` (default) or `oas-3-1` must be enabled.
//! The two are mutually exclusive: they control how the `nullable`
//! intent is encoded by the handwritten `Serialize` impls in
//! `frieze-openapi`.

#[cfg(all(feature = "oas-3-0", feature = "oas-3-1"))]
compile_error!(
    "frieze: features `oas-3-0` and `oas-3-1` are mutually exclusive; pick exactly one."
);

#[cfg(not(any(feature = "oas-3-0", feature = "oas-3-1")))]
compile_error!(
    "frieze: one of features `oas-3-0` or `oas-3-1` must be enabled (default is `oas-3-0`)."
);

mod boundary;

mod compose;
pub use compose::{compose, from_schemas};
