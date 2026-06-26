//! `Vec<Option<T>>` — array of nullable items. Under PR-F this maps to
//! `Array(Nullable(<T>))`, so the `nullable` marker rides on the **items**
//! schema and not on the outer array. The field stays in `required`.
//!
//! This shape was rejected as a compile error in PR-E; PR-F lifts the
//! restriction by moving nullability onto the type tree.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Document {
    title: String,
    sections: Vec<Option<String>>,
}

#[test]
fn vec_of_option_renders_nullable_items_under_oas_3_0() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Document>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Document:
      type: object
      properties:
        title:
          type: string
        sections:
          type: array
          items:
            type: string
            nullable: true
      required:
        - title
        - sections
    "###);
}
