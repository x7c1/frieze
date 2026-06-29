//! The [`Schema`] trait that user types implement (typically through the
//! derive macro in `frieze-macros`).

use crate::schemas_builder::SchemasBuilder;

/// Trait implemented by types that can be expressed as an OpenAPI schema.
///
/// `#[derive(frieze::Schema)]` generates an implementation of this trait.
///
/// # `name()` returns an owned `String`
///
/// The schema name is owned (`String`) rather than `&'static str` so
/// that generic types can compose names from their type arguments at
/// monomorphization time — e.g. `Box<i64>` returns
/// `format!("{}_Box", <i64 as Schema>::name())` which cannot be a
/// `&'static str`. Non-generic types simply return `"User".to_string()`;
/// the allocation happens once per schema-construction call.
///
/// # Transitive registration via [`Schema::register_into`]
///
/// [`Schema::register_into`] inserts `Self::schema()` into a
/// [`SchemasBuilder`] and is overridden by `#[derive(Schema)]` to walk
/// each field type's `register_into` so dependencies are collected
/// automatically (`SchemasBuilder::add::<Foo>()` pulls in `User`,
/// `Page<Bar>`, ... transitively when `Foo`'s fields name them).
///
/// The default impl pushes only `Self` and does **not** recurse — types
/// with a hand-written `impl Schema` must either register their own
/// dependencies (via [`SchemasBuilder::add`] / [`SchemasBuilder::push_unique`])
/// or override `register_into` themselves. This keeps the default
/// graceful for manual impls without making the trait method mandatory.
pub trait Schema {
    /// The schema name used as the key under `#/components/schemas`.
    fn name() -> String;

    /// Builds the validated domain representation of this schema.
    fn schema() -> frieze_model::Schema;

    /// Registers `Self`'s schema into `builder` along with any types
    /// that `Self::schema()` references transitively.
    ///
    /// `#[derive(Schema)]` overrides this method to walk each field
    /// type's `register_into`, producing a transitive closure rooted at
    /// `Self`. Calls are idempotent: a type whose name is already
    /// registered is skipped, so recursive types (`struct Tree {
    /// children: Vec<Box<Tree>> }`) terminate naturally.
    ///
    /// The default impl pushes only `Self::schema()` via
    /// [`SchemasBuilder::push_unique`] and does **not** recurse. Manual
    /// `impl Schema` types that need their dependencies auto-collected
    /// must override this method.
    fn register_into(builder: &mut SchemasBuilder) {
        builder.push_unique(Self::schema());
    }
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

/// Marker trait implemented by types whose [`Schema`] is meant to be
/// registered under `#/components/schemas`. Used by
/// [`crate::SchemasBuilder::add`] to gate registration at compile time:
/// primitive scalars (`i32`, `i64`, ... `String`) implement [`Schema`]
/// so they can appear as generic arguments (`Box<i64>`,
/// `Page<String>`), but they intentionally do **not** implement
/// `IsRegistrable` so `Schemas::add::<i64>()` fails to compile.
///
/// `#[derive(Schema)]` emits `impl IsRegistrable` for `struct` and
/// `enum` inputs alongside the [`Schema`] impl. Blanket impls in
/// `frieze-usecase` propagate the marker through transparent wrappers
/// (`Box<T>` / `Rc<T>` / `Arc<T>`).
///
/// Users writing a manual `impl Schema` for a struct or enum type must
/// also `impl IsRegistrable` to make the type registrable on a
/// [`crate::SchemasBuilder`].
#[diagnostic::on_unimplemented(
    message = "frieze: `{Self}` cannot be added to a `Schemas` collection directly",
    label = "this type does not implement `IsRegistrable`",
    note = "primitive scalars (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `String`) implement `Schema` so they can appear as generic arguments (e.g. `Box<i64>`, `Page<String>`) but are not registrable as standalone schemas. Wrap the scalar in a `#[derive(Schema)]` struct if you want it to appear under `#/components/schemas`, or register the wrapping type that contains this field instead."
)]
pub trait IsRegistrable: Schema {}
