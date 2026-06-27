//! Plain Rust representation of the OpenAPI Specification.
//!
//! This crate intentionally has no knowledge of `frieze-model` or
//! `frieze-usecase`. It mirrors the shape of the OAS spec and nothing more.
//!
//! # Feature flags
//!
//! Exactly one OAS version feature must be enabled. The two are mutually
//! exclusive because the OAS 3.0 and 3.1 nullability encodings cannot be
//! produced by the same renderer:
//!
//! - `oas-3-0` (default): nullable encoded as `nullable: true`.
//! - `oas-3-1`: nullable encoded as a 2-element `type` sequence
//!   (e.g. `type: [string, "null"]`).

#[cfg(all(feature = "oas-3-0", feature = "oas-3-1"))]
compile_error!(
    "frieze: features `oas-3-0` and `oas-3-1` are mutually exclusive; pick exactly one."
);

#[cfg(not(any(feature = "oas-3-0", feature = "oas-3-1")))]
compile_error!(
    "frieze: one of features `oas-3-0` or `oas-3-1` must be enabled (default is `oas-3-0`)."
);

mod object_schema;
pub use object_schema::ObjectSchema;

mod string_enum_schema;
pub use string_enum_schema::StringEnumSchema;

mod schema_object;
pub use schema_object::SchemaObject;

mod schema_type;
pub use schema_type::SchemaType;
