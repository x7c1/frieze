//! When a struct references a generic-enum instantiation
//! (`Event<i64, String>`) that has not been added to the builder,
//! `Schemas::build()` returns `Err(UnresolvedReference(...))` carrying
//! the composed schema name (`Int64_String_Event`) — the same
//! suffix-form composition rule used everywhere else.

use frieze::{Error, Schema, SchemaName};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Event<T, U> {
    Held(Container<T>),
    Lost(Container<U>),
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Outer {
    event: Event<i64, String>,
}

#[test]
fn build_fails_when_generic_enum_instance_missing() {
    let err = frieze::schemas()
        .add::<Outer>()
        .add::<Container<i64>>()
        .add::<Container<String>>()
        .build()
        .expect_err("expected an unresolved-reference error");

    assert_eq!(
        err,
        Error::UnresolvedReference(SchemaName::new("Int64_String_Event").unwrap())
    );
}
