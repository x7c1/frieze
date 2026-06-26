//! The error type used by the domain model.

use thiserror::Error;

use crate::schema_name::SchemaName;

/// Errors that can occur while constructing domain types or while collecting
/// schemas into [`crate::Schemas`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("schema name must not be empty")]
    EmptySchemaName,
    #[error(
        "schema name `{0}` must match the OpenAPI component name pattern \
         `^[a-zA-Z0-9._-]+$`"
    )]
    InvalidSchemaName(String),
    #[error("property name must not be empty")]
    EmptyPropertyName,
    #[error("schema `{0}` has no properties")]
    NoProperties(String),
    #[error("schema `{schema}` declares duplicate property `{property}`")]
    DuplicateProperty { schema: String, property: String },
    #[error("schema `{0}` was added more than once")]
    DuplicateSchema(SchemaName),
    #[error("schema `{0}` is referenced but not registered")]
    UnresolvedReference(SchemaName),
}
