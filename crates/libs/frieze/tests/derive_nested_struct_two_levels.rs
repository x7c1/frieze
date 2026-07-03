//! Two-level nesting: `Workspace` owns `Profile`, which owns `User`. All
//! three schemas appear under `#/components/schemas`, ordered
//! alphabetically (Profile, User, Workspace).
//!
//! Demonstrates that `$ref` resolution is transitive: registering the
//! three explicitly is enough for `build()` to succeed.

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
    user: User,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Workspace {
    owner: Profile,
}

#[test]
fn two_level_nested_renders_all_three() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Workspace>()
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
        User:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        Workspace:
          type: object
          required:
          - owner
          properties:
            owner:
              $ref: '#/components/schemas/Profile'
    ");
}
