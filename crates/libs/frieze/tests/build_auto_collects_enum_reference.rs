//! A struct field typed by a derived enum is auto-registered through
//! the derived `Register::register_into`: adding only the struct root
//! `User` pulls in `Status` without a second `add::<Status>()` call.
//!
//! This used to assert `Error::UnresolvedReference(Status)`; under
//! transitive `register_into` the enum reference resolves
//! automatically. The unresolved-reference path is still exercised at
//! the unit-test level in `frieze-usecase` via a hand-written `impl
//! Schema` whose default `register_into` does not recurse.

use frieze::Schema;

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
fn add_root_auto_collects_enum_reference() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("derived `register_into` auto-registers `Status` through the field walk");

    let names: Vec<&str> = s
        .by_name
        .keys()
        .map(frieze_model::SchemaName::as_str)
        .collect();
    assert_eq!(names, vec!["Status", "User"]);
}
