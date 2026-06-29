//! When a struct references a generic instantiation (`Container<User>`)
//! that has not been added to the builder, `Schemas::build()` returns
//! `Err(UnresolvedReference("User_Container"))` — the missing name
//! follows the same suffix-form composition rule used everywhere else.

use frieze::{Error, Schema, SchemaName};

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    container: Container<User>,
}

#[test]
fn build_fails_with_user_container_when_generic_instance_missing() {
    let err = frieze::schemas()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect_err("expected an unresolved-reference error");

    assert_eq!(
        err,
        Error::UnresolvedReference(SchemaName::new("User_Container").unwrap())
    );
}
