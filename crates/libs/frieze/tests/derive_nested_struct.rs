//! A struct field whose type is itself a `Schema`-deriving struct renders
//! as `$ref: "#/components/schemas/<Name>"` and the referenced schema
//! itself appears under `#/components/schemas`.
//!
//! The output for a plain (non-nullable, non-array) reference is
//! identical under both `oas-3-0` and `oas-3-1`, so this test runs under
//! either feature flag.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    name: String,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    user: User,
}

#[test]
fn nested_struct_renders_as_ref() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Profile:
      type: object
      properties:
        user:
          $ref: "#/components/schemas/User"
      required:
        - user
    User:
      type: object
      properties:
        id:
          type: integer
          format: int64
        name:
          type: string
      required:
        - id
        - name
    "###);
}
