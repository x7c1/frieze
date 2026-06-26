//! Explicitly registering every level of a nested schema graph makes
//! `build()` succeed and surfaces all three schemas under
//! `#/components/schemas`. The test deliberately registers in a
//! *different* order than the build target's alphabetical output to
//! confirm that registration order does not affect resolution.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    user: User,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Workspace {
    owner: Profile,
}

#[test]
fn explicit_transitive_registration_resolves_all_refs() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Workspace>()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("explicit registration should resolve all references");

    let names: Vec<&str> = s.by_name.keys().map(frieze::SchemaName::as_str).collect();
    assert_eq!(names, vec!["Profile", "User", "Workspace"]);
}
