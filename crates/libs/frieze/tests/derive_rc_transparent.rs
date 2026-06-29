//! `Rc<T>` is treated as a transparent owned wrapper, parallel to
//! `Box<T>` (see `derive_box_transparent.rs`).
//!
//! Note: the field-type integration that would let a user write
//! `struct Owner { rc: Rc<User> }` arrives in Phase 1 #11b. For #11a the
//! transparency is asserted at the trait level.

use std::rc::Rc;

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[test]
fn rc_user_name_delegates_to_inner() {
    assert_eq!(<Rc<User> as Schema>::name(), <User as Schema>::name());
    assert_eq!(<Rc<User> as Schema>::name(), "User");
}

#[test]
fn rc_user_schema_delegates_to_inner() {
    assert_eq!(<Rc<User> as Schema>::schema(), <User as Schema>::schema());
}
