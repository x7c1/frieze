//! `Vec<T>` and `Option<Vec<T>>` field shapes under OAS 3.1. Both are
//! required at the presence axis; the latter additionally folds `"null"`
//! into the outer array's `type` sequence (not into the items').

#![cfg(feature = "oas-3-1")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Tag {
    name: String,
    aliases: Vec<String>,
    parent_ids: Option<Vec<i64>>,
}

#[test]
fn vec_field_renders_as_type_array_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Tag>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    Tag:
      type: object
      required:
        - name
        - aliases
        - parent_ids
      properties:
        name:
          type: string
        aliases:
          type: array
          items:
            type: string
        parent_ids:
          type:
            - array
            - "null"
          items:
            type: integer
            format: int64
    "#);
}
