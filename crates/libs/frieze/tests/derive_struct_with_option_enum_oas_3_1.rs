//! `Option<Status>` (serde default — no `skip_serializing_if`) under
//! OAS 3.1 emits the `oneOf` + `{type: "null"}` wrap — the same wrap
//! used for nullable nested-struct references.

#![cfg(feature = "oas-3-1")]

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
    status: Option<Status>,
}

#[test]
fn option_enum_field_renders_as_nullable_ref_under_oas_3_1() {
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
        status:
          oneOf:
            - $ref: "#/components/schemas/Status"
            - type: "null"
      required:
        - status
    "###);
}
