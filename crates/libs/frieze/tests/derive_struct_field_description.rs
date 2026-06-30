//! A `///` doc-comment on a struct field surfaces as that property's
//! `description` in the emitted OAS schema. The shape is identical
//! under both feature flags.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    /// The user's display name.
    name: String,
}

#[test]
fn field_doc_comment_becomes_property_description() {
    let s: frieze::Schemas = frieze::schemas()
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
          - name
          properties:
            name:
              type: string
              description: The user's display name.
    ");
}
