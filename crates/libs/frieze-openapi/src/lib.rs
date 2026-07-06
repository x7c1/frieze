//! Plain Rust representation of the OpenAPI Specification.
//!
//! This crate intentionally has no knowledge of `frieze-model` or
//! `frieze-usecase`. It mirrors the shape of the OAS spec and nothing more.
//!
//! # OAS versions
//!
//! Both OAS 3.0.x and 3.1.x are supported by every build; the version
//! is per-document runtime data. Each [`Document`] carries a
//! [`Version`] (lifted from its `openapi:` string at parse time, or
//! passed explicitly at construction), and serialization dispatches on
//! it — including the encodings where the two versions differ:
//!
//! - OAS 3.0: nullable encoded as `nullable: true`, `$ref` companions
//!   wrapped in `allOf`.
//! - OAS 3.1: nullable encoded as a 2-element `type` sequence
//!   (e.g. `type: [string, "null"]`) or a `oneOf` against
//!   `{type: "null"}` for references; `description` may sit next to
//!   `$ref`.

mod object_schema;
pub use object_schema::ObjectSchema;

mod string_enum_schema;
pub use string_enum_schema::StringEnumSchema;

mod one_of_schema;
pub use one_of_schema::{OneOfSchema, OneOfVariant};

mod schema_object;
pub use schema_object::SchemaObject;

mod schema_type;
pub use schema_type::SchemaType;

mod components;
pub use components::Components;

mod info;
pub use info::Info;

mod document;
pub use document::Document;

mod serialize;

mod version;
pub use version::{Version, VersionParseError};

mod to_yaml;
pub use to_yaml::to_yaml;
