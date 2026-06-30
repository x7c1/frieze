//! Generic internal-tagged enum whose type arguments are user-defined
//! structs (rather than primitives). Verifies that the suffix-form
//! schema name composition works for any `T: Schema`, not just
//! primitives, and that the inner generic struct's `$ref` resolves to
//! the registered user struct entries (no inline expansion for
//! user-defined types).
//!
//! The shape is identical under `oas-3-0` and `oas-3-1`.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct UserA {
    id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct UserB {
    name: String,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Either<T, U> {
    Left(Container<T>),
    Right(Container<U>),
}

#[test]
fn either_two_user_structs_uses_suffix_form() {
    assert_eq!(
        <Either<UserA, UserB> as Schema>::name(),
        "UserA_UserB_Either"
    );
}

#[test]
fn either_user_structs_render_through_refs() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Either<UserA, UserB>>()
        .add::<Container<UserA>>()
        .add::<Container<UserB>>()
        .add::<UserA>()
        .add::<UserB>()
        .build()
        .expect("schemas build resolves all transitive references");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        UserA:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        UserA_Container:
          type: object
          required:
          - value
          properties:
            value:
              $ref: '#/components/schemas/UserA'
        UserA_UserB_Either:
          oneOf:
          - allOf:
            - $ref: '#/components/schemas/UserA_Container'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Left
          - allOf:
            - $ref: '#/components/schemas/UserB_Container'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Right
          discriminator:
            propertyName: kind
        UserB:
          type: object
          required:
          - name
          properties:
            name:
              type: string
        UserB_Container:
          type: object
          required:
          - value
          properties:
            value:
              $ref: '#/components/schemas/UserB'
    ");
}
