//! A `///` doc-comment on a struct surfaces as the top-level
//! `description` of the registered schema, alongside `type: object`.

use frieze::Schema;

mod common;

/// A registered user of the system.
#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[test]
fn struct_doc_comment_becomes_top_level_description() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        User:
          type: object
          description: A registered user of the system.
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
    ");
}
