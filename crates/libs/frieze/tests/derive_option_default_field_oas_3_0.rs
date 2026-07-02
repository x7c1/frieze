//! `Option<T>` without `#[serde(skip_serializing_if)]` produces the
//! required + nullable shape (`Option<T>` serde default in
//! `docs/field-shapes.md`). The field name stays in the `required` array;
//! the value-level `nullable` marker is emitted on the inner schema.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    nickname: Option<String>,
}

#[test]
fn option_default_renders_required_and_nullable_under_oas_3_0() {
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
          - nickname
          properties:
            id:
              type: integer
              format: int64
            nickname:
              type: string
              nullable: true
    ");
}
