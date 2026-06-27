//! `Option<User>` (serde default — no `skip_serializing_if`) maps to
//! branch ② (required + nullable). Under OAS 3.1, a nullable reference
//! is expressed with `oneOf: [$ref, {type: "null"}]` because the
//! `nullable` keyword was dropped in 3.1.

#![cfg(feature = "oas-3-1")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    user: Option<User>,
}

#[test]
fn option_nested_renders_as_nullable_ref_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Profile:
      type: object
      required:
        - user
      properties:
        user:
          oneOf:
            - $ref: "#/components/schemas/User"
            - type: "null"
    User:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}
