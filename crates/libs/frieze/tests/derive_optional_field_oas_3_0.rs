#![cfg(feature = "oas-3-0")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    nickname: Option<String>,
}

#[test]
fn optional_field_emits_nullable_true_under_oas_3_0() {
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
    "###);
}
