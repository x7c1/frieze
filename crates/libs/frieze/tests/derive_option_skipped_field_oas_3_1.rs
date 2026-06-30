//! `Option<T>` paired with `#[serde(skip_serializing_if = "Option::is_none")]`
//! maps to branch ③ (optional + non-nullable). Under OAS 3.1 this means the
//! field is dropped from `required` **and** the `type` stays as a scalar
//! (no `"null"` fold).

#![cfg(feature = "oas-3-1")]

use frieze::Schema;
use serde::Serialize;

mod common;

#[derive(Schema, Serialize)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
}

#[test]
fn option_with_skip_serializing_if_renders_optional_non_nullable_under_oas_3_1() {
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
          - id
          properties:
            id:
              type: integer
              format: int64
            nickname:
              type: string
    ");
}
