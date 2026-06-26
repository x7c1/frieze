//! `Option<T>` without `#[serde(skip_serializing_if)]` maps to branch ②
//! (required + nullable) under the PR-F mapping. The field name stays in
//! the `required` array; the value-level `nullable` marker is emitted on
//! the inner schema.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    nickname: Option<String>,
}

#[test]
fn option_default_renders_required_and_nullable_under_oas_3_0() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    User:
      type: object
      properties:
        id:
          type: integer
          format: int64
        nickname:
          type: string
          nullable: true
      required:
        - id
        - nickname
    "###);
}
