//! Use cases for frieze.
//!
//! Defines the [`Schema`] trait that user types implement (typically through
//! the derive macro in `frieze-macros`), the [`SchemasBuilder`] that collects
//! schemas into a validated [`frieze_model::Schemas`], and the boundary
//! conversion from `frieze-model` to `frieze-openapi` (see [`to_value`] and
//! [`to_yaml`]).
//!
//! # Feature flags
//!
//! Exactly one of `oas-3-0` (default) or `oas-3-1` must be enabled.
//! The two are mutually exclusive: they control how [`to_value`] renders
//! the `nullable` intent into YAML.

#[cfg(all(feature = "oas-3-0", feature = "oas-3-1"))]
compile_error!(
    "frieze: features `oas-3-0` and `oas-3-1` are mutually exclusive; pick exactly one."
);

#[cfg(not(any(feature = "oas-3-0", feature = "oas-3-1")))]
compile_error!(
    "frieze: one of features `oas-3-0` or `oas-3-1` must be enabled (default is `oas-3-0`)."
);

mod schema;
pub use schema::{IsRegistrable, IsStructSchema, Schema};

mod primitive_schema_impls;
mod wrapper_impls;

mod schemas_builder;
pub use schemas_builder::SchemasBuilder;

#[cfg(feature = "inventory")]
mod inventory;
#[cfg(feature = "inventory")]
pub use inventory::SchemaRoot;

mod to_value;
pub use to_value::to_value;

mod to_yaml;
pub use to_yaml::to_yaml;
