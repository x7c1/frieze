//! Domain types whose invariants are enforced by the type system.
//!
//! Types here use private fields and `pub fn new(...)` constructors so they
//! cannot be built via struct literals from outside the crate. The names are
//! intentionally bare (`Schema`, `Property`, etc.) — domain types in this
//! crate ARE the validated form, so no `Validated-` prefix is used.

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

mod schema_name;
pub use schema_name::SchemaName;

mod error;
pub use error::Error;
