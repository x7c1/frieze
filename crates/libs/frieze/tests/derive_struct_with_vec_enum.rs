//! A `Vec<Status>` field renders as `type: array, items: {$ref}`. The
//! shape is identical under both `oas-3-0` and `oas-3-1` (no
//! nullability is involved).

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    statuses: Vec<Status>,
}

#[test]
fn vec_of_enum_field_renders_as_array_of_refs() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      enum:
        - Active
        - Inactive
    User:
      type: object
      properties:
        statuses:
          type: array
          items:
            $ref: "#/components/schemas/Status"
      required:
        - statuses
    "###);
}
