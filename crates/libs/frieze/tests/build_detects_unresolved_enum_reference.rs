//! `SchemasBuilder::build()` reports an unresolved enum reference the
//! same way it reports an unresolved struct reference: via
//! `Error::UnresolvedReference(SchemaName)`. The walker descends
//! through the property type tree so a missing enum referenced from a
//! struct field is detected at build time rather than silently
//! emitting a broken `$ref`.

use frieze::{Error, Schema, SchemaName};

#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    status: Status,
}

#[test]
fn build_fails_when_referenced_enum_is_not_registered() {
    // `User` references `Status` via the enum-typed field, but we only
    // register `User`. The builder should detect that `Status` is missing.
    let err = frieze::schemas()
        .add::<User>()
        .build()
        .expect_err("expected an unresolved-reference error");

    assert_eq!(
        err,
        Error::UnresolvedReference(SchemaName::new("Status").unwrap())
    );
}
