//! Blanket [`Schema`] / [`IsStructSchema`] / [`IsRegistrable`]
//! implementations for owned wrapper types from `std`: [`Box`],
//! [`std::rc::Rc`], and [`std::sync::Arc`].
//!
//! These wrappers are **transparent** with respect to the OAS schema:
//! `<Box<User> as Schema>::name()` returns `"User"`, and
//! `<Box<User> as Schema>::schema()` returns the same schema as
//! `<User as Schema>::schema()`. The rationale is twofold:
//!
//! 1. serde serializes `Box<T>` / `Rc<T>` / `Arc<T>` as the wire form of
//!    `T`, so the OAS schema must agree.
//! 2. **Recursive types require [`Box`]**: `struct Tree { children:
//!    Vec<Box<Tree>> }` cannot be expressed in Rust without an
//!    indirection, but the OAS schema is a self-reference
//!    `$ref: '#/components/schemas/Tree'`. Treating `Box<Tree>` as a
//!    new schema entry `Tree_Box` would generate an infinite cascade of
//!    `Tree_Box_Box`, `Tree_Box_Box_Box`, ... — the transitive-closure
//!    walker would never terminate.
//!
//! `Cell<T>` / `RefCell<T>` / `Mutex<T>` / `RwLock<T>` are intentionally
//! **not** covered here. They are interior-mutability primitives that
//! rarely appear in serialisable API shapes; if a future use case
//! emerges, the blanket impl pattern in this file is the template to
//! extend.

use std::rc::Rc;
use std::sync::Arc;

use crate::schema::{IsRegistrable, IsStructSchema, Schema};
use crate::schemas_builder::SchemasBuilder;

impl<T: Schema + ?Sized> Schema for Box<T> {
    fn name() -> String {
        <T as Schema>::name()
    }
    fn schema() -> frieze_model::Schema {
        <T as Schema>::schema()
    }
    fn register_into(builder: &mut SchemasBuilder) {
        // Transparent wrappers share the inner type's schema entry
        // (`Box<T>::name() == T::name()`), so we must funnel the
        // transitive walk to the inner type's `register_into` rather
        // than re-pushing the inner schema directly. This keeps
        // `struct Tree { children: Vec<Box<Tree>> }` terminating: the
        // recursive call eventually hits the `contains_name` guard at
        // the top of `Tree::register_into`.
        <T as Schema>::register_into(builder);
    }
}

impl<T: Schema + ?Sized> Schema for Rc<T> {
    fn name() -> String {
        <T as Schema>::name()
    }
    fn schema() -> frieze_model::Schema {
        <T as Schema>::schema()
    }
    fn register_into(builder: &mut SchemasBuilder) {
        <T as Schema>::register_into(builder);
    }
}

impl<T: Schema + ?Sized> Schema for Arc<T> {
    fn name() -> String {
        <T as Schema>::name()
    }
    fn schema() -> frieze_model::Schema {
        <T as Schema>::schema()
    }
    fn register_into(builder: &mut SchemasBuilder) {
        <T as Schema>::register_into(builder);
    }
}

impl<T: IsStructSchema + ?Sized> IsStructSchema for Box<T> {}
impl<T: IsStructSchema + ?Sized> IsStructSchema for Rc<T> {}
impl<T: IsStructSchema + ?Sized> IsStructSchema for Arc<T> {}

impl<T: IsRegistrable + ?Sized> IsRegistrable for Box<T> {}
impl<T: IsRegistrable + ?Sized> IsRegistrable for Rc<T> {}
impl<T: IsRegistrable + ?Sized> IsRegistrable for Arc<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyUser;

    impl Schema for DummyUser {
        fn name() -> String {
            "User".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "User",
                vec![frieze_model::Property::new(
                    "id",
                    frieze_model::PropertyType::Int64,
                    frieze_model::Presence::Required,
                )
                .unwrap()],
            )
            .unwrap()
        }
    }

    #[test]
    fn box_name_is_inner_name() {
        assert_eq!(<Box<DummyUser> as Schema>::name(), "User");
    }

    #[test]
    fn rc_name_is_inner_name() {
        assert_eq!(<Rc<DummyUser> as Schema>::name(), "User");
    }

    #[test]
    fn arc_name_is_inner_name() {
        assert_eq!(<Arc<DummyUser> as Schema>::name(), "User");
    }

    #[test]
    fn box_schema_equals_inner_schema() {
        assert_eq!(
            <Box<DummyUser> as Schema>::schema(),
            <DummyUser as Schema>::schema()
        );
    }

    #[test]
    fn primitive_box_name_is_primitive_name() {
        // `i64` impl lives in the sibling `primitive_schema_impls`
        // module; this exercises the blanket impl over a primitive
        // inner.
        assert_eq!(<Box<i64> as Schema>::name(), "Int64");
    }

    #[test]
    fn nested_box_is_transparent() {
        // `Box<Box<T>>` collapses to `T`'s name through the recursive
        // blanket impl — important for the recursive-type ergonomics.
        assert_eq!(<Box<Box<DummyUser>> as Schema>::name(), "User");
    }
}
