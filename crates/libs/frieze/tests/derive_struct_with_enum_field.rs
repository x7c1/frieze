//! A struct field whose type is a `Schema`-deriving unit-variant enum
//! is emitted as `$ref: "#/components/schemas/<Name>"`, with the
//! referenced enum schema registered alongside under
//! `#/components/schemas`. This is the same `$ref` transit path as
//! nested struct references — enums ride on it unchanged.
//!
//! The output for a plain (non-nullable, non-array) reference is
//! identical under both `oas-3-0` and `oas-3-1`.

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
    id: i64,
    status: Status,
}

#[test]
fn struct_field_of_enum_type_renders_as_ref() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Status:
      type: string
      enum:
        - Active
        - Inactive
    User:
      type: object
      required:
        - id
        - status
      properties:
        id:
          type: integer
          format: int64
        status:
          $ref: "#/components/schemas/Status"
    "##);
}
