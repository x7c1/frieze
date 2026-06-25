use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct User {
    id: i64,
    name: String,
}

#[test]
fn user_struct_minimum() {
    let schemas = frieze::Schemas::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(schemas.to_value(), @r###"
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
