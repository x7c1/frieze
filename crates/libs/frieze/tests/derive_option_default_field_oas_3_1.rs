//! `Option<T>` without `#[serde(skip_serializing_if)]` maps to branch ②
//! (required + nullable) under the current optionality mapping. The
//! field name stays in the `required` array; the value-level `"null"`
//! marker is folded into the inner schema's `type` sequence under
//! OAS 3.1.

#![cfg(feature = "oas-3-1")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    nickname: Option<String>,
}

#[test]
fn option_default_renders_required_and_nullable_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    User:
      type: object
      required:
        - id
        - nickname
      properties:
        id:
          type: integer
          format: int64
        nickname:
          type:
            - string
            - "null"
    "#);
}
