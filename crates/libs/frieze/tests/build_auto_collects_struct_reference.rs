//! `SchemasBuilder::add::<T>()` auto-collects every struct reachable
//! from `T`'s field types through the derived `Register::register_into`.
//! Adding only the root `Workspace` is enough — `Profile` follows
//! transitively without an explicit second `add` call.
//!
//! This used to be the negative test asserting
//! `Error::UnresolvedReference(Profile)` when `Profile` was omitted;
//! with transitive `register_into` the reference resolves
//! automatically. The unresolved-reference error path now only fires
//! for hand-written `impl Schema` types whose default `register_into`
//! does not walk dependencies, which is covered by a unit test in
//! `frieze-usecase`.

use frieze::Schema;

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
fn add_root_auto_collects_nested_struct_reference() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Workspace>()
        .build()
        .expect("derived `register_into` auto-registers `Profile` through the field walk");

    let names: Vec<&str> = s
        .by_name
        .keys()
        .map(frieze_model::SchemaName::as_str)
        .collect();
    assert_eq!(names, vec!["Profile", "Workspace"]);
}
