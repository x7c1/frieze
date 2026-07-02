//! Explicitly registering both a struct and the enum it references in
//! a field type makes `build()` succeed. Registering in a different
//! order than the build target's alphabetical output confirms that the
//! resolution does not depend on insertion order.

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
fn explicit_enum_registration_resolves_the_ref() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .add::<Status>()
        .build()
        .expect("explicit registration should resolve the enum reference");

    let names: Vec<&str> = s
        .by_name
        .keys()
        .map(frieze_model::SchemaName::as_str)
        .collect();
    assert_eq!(names, vec!["Status", "User"]);
}
