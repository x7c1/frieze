//! `Vec<User>` renders as an array whose items are a `$ref` to `User`.
//! The output is identical under both OAS versions.

use frieze::Schema;

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

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Team:
      type: object
      properties:
        members:
          type: array
          items:
            $ref: "#/components/schemas/User"
      required:
        - members
    User:
      type: object
      properties:
        id:
          type: integer
          format: int64
      required:
        - id
    "###);
}
