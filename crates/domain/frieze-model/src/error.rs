//! The error type used by the domain model.

use thiserror::Error;

/// Errors that can occur while constructing domain types or while collecting
/// schemas into [`crate::Schemas`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("schema name must not be empty")]
    EmptySchemaName,
    #[error("property name must not be empty")]
    EmptyPropertyName,
    #[error("schema `{0}` has no properties")]
    NoProperties(String),
    #[error("schema `{schema}` declares duplicate property `{property}`")]
    DuplicateProperty { schema: String, property: String },
    #[error("schema `{0}` was added more than once")]
    DuplicateSchema(String),
}
