//! `SchemasBuilder::build()` reports the first unresolved reference as
//! `Error::UnresolvedReference(SchemaName)` instead of silently emitting
//! a broken `$ref`. This is the fail-fast UX that matches
//! `Error::DuplicateSchema`.

use frieze::{Error, Schema, SchemaName};

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Workspace {
    owner: Profile,
}

#[test]
fn build_fails_when_referenced_schema_is_not_registered() {
    // `Workspace` references `Profile` via the nested-struct field, but
    // we only register `Workspace`. The builder should detect that
    // `Profile` is missing and surface it.
    let err = frieze::schemas()
        .add::<Workspace>()
        .build()
        .expect_err("expected an unresolved-reference error");

    assert_eq!(
        err,
        Error::UnresolvedReference(SchemaName::new("Profile").unwrap())
    );
}
