//! `Option<Vec<T>>` paired with `Option::is_none` is optional and
//! non-nullable, just like scalar and reference `Option<T>` fields.

use frieze::Schema;
use serde::Serialize;

mod common;

#[derive(Schema, Serialize)]
#[allow(dead_code)]
struct Filter {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

#[test]
fn option_vec_with_skip_is_optional_and_non_nullable() {
    let schemas = frieze::SchemasBuilder::new()
        .add::<Filter>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(schemas), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Filter:
          type: object
          required:
          - query
          properties:
            query:
              type: string
            tags:
              type: array
              items:
                type: string
    ");
}
