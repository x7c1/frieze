//! `Vec<Option<T>>` under OAS 3.1: the `"null"` marker rides on the
//! **items**' `type` sequence, not on the outer array. The field stays
//! in `required`.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Document {
    title: String,
    sections: Vec<Option<String>>,
}

#[test]
fn vec_of_option_renders_nullable_items_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Document>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_1(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Document:
          type: object
          required:
          - title
          - sections
          properties:
            title:
              type: string
            sections:
              type: array
              items:
                type:
                - string
                - 'null'
    ");
}
