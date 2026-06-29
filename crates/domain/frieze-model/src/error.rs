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
    #[error("schema `{0}` has no variants")]
    NoVariants(String),
    #[error("schema `{schema}` declares duplicate variant value `{value}`")]
    DuplicateVariantValue { schema: String, value: String },
    #[error("schema `{0}` declares an empty variant value")]
    EmptyVariantValue(String),
    #[error("schema `{schema}` declares duplicate property `{property}`")]
    DuplicateProperty { schema: String, property: String },
    #[error("schema `{0}` was added more than once")]
    DuplicateSchema(SchemaName),
    /// A `$ref` resolution failure detected by
    /// [`crate::Schemas`]-consuming code (typically
    /// `frieze_usecase::SchemasBuilder::build`).
    ///
    /// Reachable only through the low-level API: code that uses
    /// `#[derive(Schema)]` together with `SchemasBuilder::add` /
    /// `SchemasBuilder::push_unique` never triggers this error
    /// because the derived [`Schema::register_into`] walks each field
    /// type and registers transitive dependencies automatically.
    ///
    /// Two situations still raise the error:
    ///
    /// 1. A hand-written `impl Schema` whose `schema()` body contains
    ///    a `$ref` to another type, but that does not override
    ///    `register_into` to register the referenced type — the
    ///    default `register_into` is non-recursive.
    /// 2. A `Schemas` constructed directly from `Schemas::new(vec![...])`
    ///    with manually-assembled `Schema` values whose references do
    ///    not match any entry in the same `Vec`.
    ///
    /// Recovery is the same in both cases: register the missing type
    /// (`SchemasBuilder::add::<MissingType>()`), or override
    /// `register_into` on the manual `impl Schema` to call
    /// `<MissingType as Schema>::register_into(builder)`.
    #[error(
        "schema `{0}` is referenced but not registered (add it via \
         `SchemasBuilder::add::<...>()`, or override `register_into` \
         on the manual `impl Schema` to walk the referenced types)"
    )]
    UnresolvedReference(SchemaName),
    #[error("oneOf schema `{0}` declares an empty discriminator tag")]
    EmptyOneOfTag(String),
    #[error(
        "oneOf schema `{schema}` variant `{variant}` references `{inner}`, \
         which is not a struct schema; internal-tagged variants require \
         their inner type to be a struct"
    )]
    OneOfVariantInnerNotStruct {
        schema: String,
        variant: String,
        inner: SchemaName,
    },
    #[error(
        "scalar schema requires a leaf PropertyType (Int32 / Int64 / \
         UInt32 / UInt64 / Float / Double / Boolean / String); composite \
         variants (Array / Nullable / Reference) are not scalar"
    )]
    NonScalarPropertyType,
}
