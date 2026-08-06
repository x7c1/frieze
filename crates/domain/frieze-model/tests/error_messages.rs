//! Pins the exact user-facing wording of [`Error`] variants that carry
//! a recovery hint.
//!
//! These messages surface verbatim through the CLI, so their wording is
//! part of the user interface: any rewording must be deliberate and show
//! up in a diff of this file, not slip in as a side effect.
//!
//! A remedy must also be valid exactly as written: `#[frieze(namespace)]`
//! takes no argument, and `frieze-macros` rejects `namespace = "..."`
//! with a compile error (see its `ui/namespace_attr_with_arg` test).

use frieze_model::{Error, Presence, Property, PropertyType, Schema, SchemaName};

fn schema_name(name: &str) -> SchemaName {
    SchemaName::new(name).unwrap()
}

fn object(name: &str, property: &str) -> Schema {
    Schema::new_object(
        name,
        vec![Property::new(property, PropertyType::String, Presence::Required).unwrap()],
    )
    .unwrap()
}

#[test]
fn schema_conflict() {
    let error = Error::SchemaConflict {
        name: schema_name("User"),
        existing: Box::new(object("User", "id")),
        incoming: Box::new(object("User", "name")),
    };
    assert_eq!(
        error.to_string(),
        "schema `User` was registered twice with different definitions \
         (use `#[frieze(namespace)]` on a containing `mod` to give them \
         distinct fully-qualified names, or rename one of the types)"
    );
}

#[test]
fn reserved_schema_name() {
    let error = Error::ReservedSchemaName {
        name: schema_name("Int64"),
    };
    assert_eq!(
        error.to_string(),
        "schema `Int64` is registered under a name reserved for a \
         primitive scalar (Int32 / Int64 / UInt32 / UInt64 / Float / \
         Double / Boolean / String); references to it would be inlined \
         as that scalar instead of pointing at the schema (rename the \
         Rust type, or put it under a `#[frieze(namespace)]` `mod` so \
         its registered name carries that mod's ident as a prefix)"
    );
}
