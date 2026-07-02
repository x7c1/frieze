//! `Vec<Option<User>>` renders as an array whose items are a nullable
//! reference. Under OAS 3.0, that is `allOf: [$ref], nullable: true` per
//! item.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Team {
    members: Vec<Option<User>>,
}

#[test]
fn vec_of_option_nested_renders_array_of_nullable_ref_under_oas_3_0() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Team>()
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
        Team:
          type: object
          required:
          - members
          properties:
            members:
              type: array
              items:
                allOf:
                - $ref: '#/components/schemas/User'
                nullable: true
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
