//! The [`Schema`] trait that user types implement (typically through the
//! derive macro in `frieze-macros`).

/// Trait implemented by types that can be expressed as an OpenAPI schema.
///
/// `#[derive(frieze::Schema)]` generates an implementation of this trait.
pub trait Schema {
    /// The schema name used as the key under `#/components/schemas`.
    fn name() -> &'static str;

    /// Builds the validated domain representation of this schema.
    fn schema() -> frieze_model::Schema;
}
