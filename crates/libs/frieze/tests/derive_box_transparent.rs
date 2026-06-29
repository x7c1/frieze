//! `Box<T>` is treated as a transparent owned wrapper: the schema name
//! and the schema body delegate to the inner type. This means
//! `<Box<User> as Schema>::name() == "User"` and the schema body is
//! identical to `User`'s — i.e. no separate `User_Box` entry is
//! generated under `#/components/schemas`.
//!
//! Note: the field-type integration that would let a user write
//! `struct Owner { boxed: Box<User> }` arrives in Phase 1 #11b (field
//! type parsing of generic arguments). For #11a the transparency is
//! asserted at the trait level instead — sufficient to lock in the
//! contract that #11b builds upon.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[test]
fn box_user_name_delegates_to_inner() {
    assert_eq!(<Box<User> as Schema>::name(), <User as Schema>::name());
    assert_eq!(<Box<User> as Schema>::name(), "User");
}

#[test]
fn box_user_schema_delegates_to_inner() {
    assert_eq!(<Box<User> as Schema>::schema(), <User as Schema>::schema());
}

#[test]
fn box_box_user_collapses_to_user() {
    // `Box<Box<User>>` is also transparent through the recursive
    // blanket impl: same name, same schema body. This guarantees that
    // composing owned wrappers never produces synthetic schema names
    // like `User_Box_Box`.
    assert_eq!(<Box<Box<User>> as Schema>::name(), "User");
    assert_eq!(
        <Box<Box<User>> as Schema>::schema(),
        <User as Schema>::schema()
    );
}
