//! `Option<User>` with `#[serde(skip_serializing_if = "Option::is_none")]`
//! maps to branch ③: optional + non-nullable. The field is dropped from
//! `required` and the referenced schema is emitted as a plain `$ref`
//! (no `allOf` / `oneOf` wrap, since the value is not nullable on the
//! wire). Identical output under both OAS versions.

use frieze::Schema;
use serde::Serialize;

#[derive(Schema, Serialize)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema, Serialize)]
#[allow(dead_code)]
struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<User>,
}

#[test]
fn option_nested_with_skip_renders_plain_ref_and_optional_presence() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Profile:
      type: object
      properties:
        user:
          $ref: "#/components/schemas/User"
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
