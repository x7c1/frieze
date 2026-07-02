//! A doc-comment with only whitespace (blank line, spaces, tabs) does
//! not surface as a `description` key — the empty-container omission
//! rule covers descriptions just like it does `required` / `properties`.

// The empty `///` lines are intentional: the test is asserting that
// they do not surface as a `description` in the OAS schema.
#![allow(clippy::empty_docs)]

use frieze::Schema;

mod common;

///
///
#[derive(Schema)]
#[allow(dead_code)]
struct User {
    ///
    id: i64,
}

#[test]
fn whitespace_only_doc_comment_emits_no_description_key() {
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
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
    ");
}
