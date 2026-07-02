//! The [`Schema`] trait that user types implement (typically through the
//! derive macro in `frieze-macros`).

/// Trait implemented by types that can be expressed as an OpenAPI schema.
///
/// `#[derive(frieze::Schema)]` generates an implementation of this trait.
///
/// `Schema` is the pure "type → schema value" contract: it carries no
/// side effects. Registration into a [`crate::SchemasBuilder`] —
/// including the transitive walk over field types — lives on the
/// companion [`crate::Register`] trait.
///
/// # `name()` returns an owned `String`
///
/// The schema name is owned (`String`) rather than `&'static str` so
/// that generic types can compose names from their type arguments at
/// monomorphization time — e.g. `Box<i64>` returns
/// `format!("{}_Box", <i64 as Schema>::name())` which cannot be a
/// `&'static str`. Non-generic types simply return `"User".to_string()`;
/// the allocation happens once per schema-construction call.
pub trait Schema {
    /// The schema name used as the key under `#/components/schemas`.
    fn name() -> String;

    /// Builds the validated domain representation of this schema.
    fn schema() -> frieze_model::Schema;
}

/// Marker trait implemented by types whose [`Schema`] is a top-level
/// **struct** schema (`Schema::Object`). Used by the derive expansion to
/// gate the inner types of internally-tagged enum variants at compile
/// time, so an enum-typed inner is rejected before
/// [`crate::SchemasBuilder::build`] runs.
///
/// `#[derive(Schema)]` emits `impl IsStructSchema` for `struct` inputs
/// and **not** for `enum` inputs, so a `oneOf` variant whose inner is
/// another enum produces a compile error rather than a runtime build
/// failure. The diagnostic message is attached via
/// `#[diagnostic::on_unimplemented]` so the rustc error explains the
/// fix path verbatim.
///
/// Users writing a manual `impl Schema` must also `impl IsStructSchema`
/// if they want their type usable as the inner of a `oneOf` variant.
#[diagnostic::on_unimplemented(
    message = "frieze: internal-tagged enum variants require their inner type to be a struct schema, but `{Self}` is not",
    label = "this type does not implement `IsStructSchema`",
    note = "wrap `{Self}` in a struct with `#[derive(Schema)]` and use the wrapping struct as the newtype variant inner: `struct {Self}Data {{ value: {Self} }}`, then `enum YourEnum {{ ... ({Self}Data), ... }}`"
)]
pub trait IsStructSchema: Schema {}
