//! `Arc<T>` is treated as a transparent owned wrapper, parallel to
//! `Box<T>` (see `derive_box_transparent.rs`).
//!
//! The transparency is asserted at the trait level here.

use std::sync::Arc;

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[test]
fn arc_user_name_delegates_to_inner() {
    assert_eq!(<Arc<User> as Schema>::name(), <User as Schema>::name());
    assert_eq!(<Arc<User> as Schema>::name(), "User");
}

#[test]
fn arc_user_schema_delegates_to_inner() {
    assert_eq!(<Arc<User> as Schema>::schema(), <User as Schema>::schema());
}
