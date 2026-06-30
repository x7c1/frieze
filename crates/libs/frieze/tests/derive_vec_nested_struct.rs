//! `Vec<User>` renders as an array whose items are a `$ref` to `User`.
//! The output is identical under both OAS versions.

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
    members: Vec<User>,
}

#[test]
fn vec_of_nested_struct_renders_array_of_ref() {
    let s: frieze::Schemas = frieze::schemas()
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
                $ref: '#/components/schemas/User'
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
