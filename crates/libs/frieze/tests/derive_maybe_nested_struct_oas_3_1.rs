//! `Maybe<User>` produces the optional + nullable shape (`Maybe<U>` in
//! `docs/field-shapes.md`). The field is dropped from `required` and the
//! reference is rendered through the OAS-3.1 nullable-ref wrap
//! (`oneOf: [$ref, {type: "null"}]`).

#![cfg(feature = "oas-3-1")]

use frieze::Schema;
use frieze_model::Maybe;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Profile {
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    user: Maybe<User>,
}

#[test]
fn maybe_nested_renders_as_optional_nullable_ref_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
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
        Profile:
          type: object
          properties:
            user:
              oneOf:
              - $ref: '#/components/schemas/User'
              - type: 'null'
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
