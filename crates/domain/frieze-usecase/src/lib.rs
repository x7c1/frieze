//! Use cases for frieze.
//!
//! Defines the [`Schema`] trait that user types implement (typically through
//! the derive macro in `frieze-macros`), the [`SchemasBuilder`] that collects
//! schemas into a validated [`frieze_model::Schemas`], and the boundary
//! conversion from `frieze-model` to `frieze-openapi` (see [`to_value`] and
//! [`to_yaml`]).

mod schema;
pub use schema::Schema;

mod schemas_builder;
pub use schemas_builder::SchemasBuilder;

mod to_value;
pub use to_value::to_value;

mod to_yaml;
pub use to_yaml::to_yaml;
