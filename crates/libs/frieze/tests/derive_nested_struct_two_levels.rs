//! Two-level nesting: `Workspace` owns `Profile`, which owns `User`. All
//! three schemas appear under `#/components/schemas`, ordered
//! alphabetically (Profile, User, Workspace).
//!
//! Demonstrates that `$ref` resolution is transitive: registering the
//! three explicitly is enough for `build()` to succeed.

use frieze::Schema;

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
    let s: frieze::Schemas = frieze::schemas()
        .add::<Workspace>()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Profile:
      type: object
      properties:
        user:
          $ref: "#/components/schemas/User"
      required:
        - user
    User:
      type: object
      properties:
        id:
          type: integer
          format: int64
      required:
        - id
    Workspace:
      type: object
      properties:
        owner:
          $ref: "#/components/schemas/Profile"
      required:
        - owner
    "###);
}
