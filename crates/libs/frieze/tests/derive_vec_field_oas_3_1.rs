//! `Vec<T>` and `Option<Vec<T>>` field shapes under OAS 3.1. Both are
//! required at the presence axis; the latter additionally folds `"null"`
//! into the outer array's `type` sequence (not into the items').

#![cfg(feature = "oas-3-1")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Tag {
    name: String,
    aliases: Vec<String>,
    parent_ids: Option<Vec<i64>>,
}

#[test]
fn vec_field_renders_as_type_array_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Tag>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
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
              - 'null'
              items:
                type: integer
                format: int64
    ");
}
