//! `Vec<Option<User>>` renders as an array whose items are a nullable
//! reference. Under OAS 3.0, that is `allOf: [$ref], nullable: true` per
//! item.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

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
    let s: frieze::Schemas = frieze::schemas()
        .add::<Team>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Team:
      type: object
      required:
        - members
      properties:
        members:
          type: array
          items:
            allOf:
              - $ref: "#/components/schemas/User"
            nullable: true
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
