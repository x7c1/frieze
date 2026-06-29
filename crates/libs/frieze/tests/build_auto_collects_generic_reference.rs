//! A generic-struct instantiation (`Container<User>`) used as a
//! struct-field type is auto-registered through the derived
//! `Schema::register_into`: adding only `Profile` pulls in
//! `Container<User>` (composed name `User_Container`) and `User`
//! without explicit second / third `add` calls.
//!
//! This used to assert `Error::UnresolvedReference(User_Container)`;
//! under transitive `register_into` the generic-instance reference
//! resolves automatically. Each derive-site's `register_into` walks
//! its syntactically-visible field types, including the concrete
//! `Container<User>` instantiation, so the monomorphic
//! `<Container<User> as Schema>::register_into` fires at runtime and
//! registers itself plus `User`.

use frieze::Schema;

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
fn add_root_auto_collects_generic_instance() {
    let s: frieze::Schemas = frieze::schemas().add::<Profile>().build().expect(
        "derived `register_into` auto-registers `Container<User>` and `User` \
             through the field walk",
    );

    let names: Vec<&str> = s.by_name.keys().map(frieze::SchemaName::as_str).collect();
    assert_eq!(names, vec!["Profile", "User", "User_Container"]);
}
