//! Plain Rust representation of the OpenAPI Specification.
//!
//! This crate intentionally has no knowledge of `frieze-model` or
//! `frieze-usecase`. It mirrors the shape of the OAS spec and nothing more.

mod schema_object;
pub use schema_object::SchemaObject;

mod schema_type;
pub use schema_type::SchemaType;
