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
    /// Two schemas were registered under the same name but with
    /// different content.
    ///
    /// Reachable through `frieze_usecase::SchemasBuilder::push_unique`
    /// (and therefore through `SchemasBuilder::add`) when two roots
    /// produce the same registration name but disagree on their body —
    /// e.g. two distinct Rust types renamed to the same OAS name, or a
    /// hand-written `impl Schema` colliding with a derived one. Same-name
    /// registrations whose bodies are byte-for-byte equal are still
    /// silently deduplicated; only divergence surfaces this error.
    ///
    /// The error is reported by `SchemasBuilder::build`, not at the
    /// moment of registration — `push_unique` records the first conflict
    /// and the builder fails fast when finalized. The `existing` and
    /// `incoming` schemas are wrapped in `Box` to keep `Error`'s size
    /// down; their bodies are available for inspection or error
    /// rendering.
    ///
    /// Recovery: rename one of the colliding types, or attach
    /// `#[frieze(namespace = "...")]` to a containing `mod` so the two
    /// types acquire distinct fully-qualified registration names.
    #[error(
        "schema `{name}` was registered twice with different definitions \
         (use `#[frieze(namespace = ...)]` on a containing `mod` to give \
         them distinct fully-qualified names, or rename one of the types)"
    )]
    SchemaConflict {
        name: SchemaName,
        existing: Box<crate::Schema>,
        incoming: Box<crate::Schema>,
    },
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
    /// The partial OAS document handed to `compose` already contains
    /// schemas under `components.schemas`. That slot must be empty so
    /// that schemas generated from Rust types via `#[derive(Schema)]`
    /// are the single source of truth.
    ///
    /// Recovery: remove every entry from `components.schemas` in the
    /// partial document and re-run `compose` — the schemas registered
    /// via `SchemasBuilder` will fill the slot.
    #[error(
        "partial OAS document already contains {count} schema(s) under \
         `components.schemas`; this slot must be empty so that schemas \
         generated from Rust types via `#[derive(Schema)]` are the \
         single source of truth. Remove the schemas from your partial \
         OAS document."
    )]
    PartialAlreadyHasSchemas { count: usize },
    /// The OpenAPI document's `openapi` field is missing or empty. This
    /// error surfaces from the composition entry points when the caller
    /// hands them a document that does not carry an OAS version at all
    /// — the version discriminant is required for shape dispatch.
    #[error("OpenAPI document is missing required 'openapi' field")]
    MissingOasVersion,
    /// The OpenAPI document's `openapi` field has an unsupported value,
    /// or the caller asked for an OAS version that this build cannot
    /// emit. Currently supported: `3.0.x`, `3.1.x`.
    ///
    /// Emitted by the composition entry points when the caller's
    /// requested [`crate::Schema`]-serialising version does not match
    /// the OAS version this crate build was compiled for. The active
    /// build's serializers are cfg-gated on the `oas-3-0` / `oas-3-1`
    /// features, so a mismatch would produce inconsistent output —
    /// this error catches that before serialization begins.
    #[error("Unsupported OAS version: '{got}'. Supported: 3.0.x, 3.1.x")]
    UnsupportedOasVersion { got: String },
}
