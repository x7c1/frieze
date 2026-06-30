//! Same source as `derive_struct_with_ref_description_oas_3_0` but the
//! OAS 3.1 build allows `$ref` to carry sibling keys directly — the
//! `description` sits next to the `$ref` with no wrap.

#![cfg(feature = "oas-3-1")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    /// The user account this profile belongs to.
    user: User,
}

#[test]
fn ref_field_with_description_emits_sibling_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
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
          required:
          - user
          properties:
            user:
              $ref: '#/components/schemas/User'
              description: The user account this profile belongs to.
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
