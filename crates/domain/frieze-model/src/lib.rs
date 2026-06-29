//! Domain types whose invariants are enforced at construction time.
//!
//! Data-structure types here (`Schema`, `Schemas`, `Property`) expose `pub`
//! fields and provide a `pub fn new(...)` constructor that validates input.
//! The constructor is the single guarantee point: a value returned by `new`
//! satisfies the documented invariants. Once constructed, the value is not
//! deep-frozen — a caller may mutate via the `pub` fields, in which case
//! upholding the invariants is the caller's responsibility. This follows
//! the Rust idiom of using struct fields as the read/write surface for
//! types whose contract is shape rather than behavior.
//!
//! Newtype wrappers around `String` (`SchemaName`, `PropertyName`) keep
//! their inner field private and expose `as_str` / `into_string` /
//! `AsRef<str>` instead — the newtype boundary is the contract.
//!
//! The names are intentionally bare (`Schema`, `Property`, etc.) — domain
//! types in this crate ARE the validated form, so no `Validated-` prefix
//! is used.
//!
//! [`Maybe`] is the one type here that is not "validated domain data" but
//! a value-type primitive: it expresses the three-state "missing / null /
//! present" distinction that frieze maps to OAS optional + nullable, and
//! that is independently useful for serde-driven Rust code (e.g. HTTP
//! `PATCH` request bodies).

mod description;

mod object_schema;
pub use object_schema::ObjectSchema;

mod string_enum_schema;
pub use string_enum_schema::StringEnumSchema;

mod one_of_schema;
pub use one_of_schema::{OneOfSchema, OneOfVariant};

mod scalar_schema;
pub use scalar_schema::ScalarSchema;

mod schema;
pub use schema::Schema;

mod schemas;
pub use schemas::Schemas;

mod property;
pub use property::Property;

mod property_name;
pub use property_name::PropertyName;

mod property_type;
pub use property_type::PropertyType;

mod presence;
pub use presence::Presence;

mod maybe;
pub use maybe::Maybe;

mod schema_name;
pub use schema_name::SchemaName;

mod error;
pub use error::Error;
