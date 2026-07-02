//! A generic-enum instantiation (`Event<i64, String>`) used as a
//! struct-field type is auto-registered through the derived
//! `Schema::register_into`: adding only the root `Outer` is enough
//! for the builder to discover the composed enum schema
//! (`Int64_String_Event`) plus every variant-inner type
//! (`Container<i64>`, `Container<String>`).
//!
//! This used to assert `Error::UnresolvedReference(Int64_String_Event)`;
//! under transitive `register_into` the enum-instance reference resolves
//! automatically. The enum derive emits a `register_into` body that
//! recurses into each variant's inner type, so the monomorphic
//! `<Event<i64, String> as Schema>::register_into` fires at runtime
//! and registers `Container<i64>` and `Container<String>`.

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
fn add_root_auto_collects_generic_enum_instance() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new().add::<Outer>().build().expect(
        "derived `register_into` auto-registers `Event<i64, String>` and its \
             variant-inner instantiations through the field walk",
    );

    let names: Vec<&str> = s
        .by_name
        .keys()
        .map(frieze_model::SchemaName::as_str)
        .collect();
    assert_eq!(
        names,
        vec![
            "Int64_Container",
            "Int64_String_Event",
            "Outer",
            "String_Container",
        ]
    );
}
