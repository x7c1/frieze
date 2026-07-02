//! `Schemas::build()` resolves a `$ref` to a generic-enum
//! instantiation (`Event<i64, String>` → `Int64_String_Event`) when
//! that instantiation and the inner generic structs are registered,
//! the same way it resolves any other cross-schema reference.
//!
//! Primitive arguments inside the inner structs (`Container<i64>` ->
//! `value: i64` → `Reference("Int64")`) are inlined at the leaf
//! position via the shared primitive-inline path, so no extra
//! `Schemas::add::<i64>()` is required (and indeed it's a compile
//! error per the `IsRegistrable` design).

use frieze::Schema;
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
fn build_resolves_reference_to_generic_enum_instance() {
    let _ = frieze::SchemasBuilder::new()
        .add::<Outer>()
        .add::<Event<i64, String>>()
        .add::<Container<i64>>()
        .add::<Container<String>>()
        .build()
        .expect("schemas build resolves the generic-enum reference and its transitive closure");
}
