//! The [`Register`] trait: how a [`Schema`] type registers itself (and
//! its transitive dependencies) into a [`SchemasBuilder`].
//!
//! [`crate::Schema`] is the pure "type → schema value" contract;
//! `Register` is the registration contract layered on top of it. The
//! split keeps the side-effecting builder walk out of the pure trait:
//! `Schema::name()` / `Schema::schema()` never touch a builder, while
//! `Register::register_into` exists solely to push schemas into one.

use crate::schema::Schema;
use crate::schemas_builder::SchemasBuilder;

/// Registration contract for [`Schema`] types.
///
/// `#[derive(Schema)]` implements this trait alongside [`Schema`],
/// overriding [`Register::register_into`] to walk each field type's
/// `register_into` so dependencies are collected automatically
/// (`SchemasBuilder::add::<Foo>()` pulls in `User`, `Page<Bar>`, ...
/// transitively when `Foo`'s fields name them).
///
/// The default impl pushes only `Self` and does **not** recurse — types
/// with a hand-written `impl Schema` add an `impl Register` (an empty
/// block picks up the default) and must either register their own
/// dependencies (via [`SchemasBuilder::add`] /
/// [`SchemasBuilder::push_unique`]) or override `register_into`
/// themselves. This keeps the default graceful for manual impls without
/// making the trait method mandatory.
pub trait Register: Schema {
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
    /// `impl Register` types that need their dependencies
    /// auto-collected must override this method.
    fn register_into(builder: &mut SchemasBuilder) {
        builder.push_unique(Self::schema());
    }
}

/// Marker trait implemented by types whose [`Schema`] is meant to be
/// registered under `#/components/schemas`. Used by
/// [`crate::SchemasBuilder::add`] to gate registration at compile time:
/// primitive scalars (`i32`, `i64`, ... `String`) implement [`Schema`]
/// and [`Register`] so they can appear as generic arguments
/// (`Box<i64>`, `Page<String>`), but they intentionally do **not**
/// implement `IsRegistrable` so `Schemas::add::<i64>()` fails to
/// compile.
///
/// `#[derive(Schema)]` emits `impl IsRegistrable` for `struct` and
/// `enum` inputs alongside the [`Schema`] / [`Register`] impls. Blanket
/// impls propagate the marker through transparent wrappers
/// (`Box<T>` / `Rc<T>` / `Arc<T>`).
///
/// Users writing a manual `impl Schema` for a struct or enum type must
/// also `impl Register` and `impl IsRegistrable` to make the type
/// registrable on a [`crate::SchemasBuilder`].
#[diagnostic::on_unimplemented(
    message = "frieze: `{Self}` cannot be added to a `Schemas` collection directly",
    label = "this type does not implement `IsRegistrable`",
    note = "primitive scalars (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `String`) implement `Schema` so they can appear as generic arguments (e.g. `Box<i64>`, `Page<String>`) but are not registrable as standalone schemas. Wrap the scalar in a `#[derive(Schema)]` struct if you want it to appear under `#/components/schemas`, or register the wrapping type that contains this field instead."
)]
pub trait IsRegistrable: Register {}
