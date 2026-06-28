//! Top-level and per-field descriptions coexist: the struct doc lands on
//! the schema's top-level `description`, each field's doc lands on its
//! property schema. Order is independent — every `description` lives
//! between `type` and the rest of its containing schema.

use frieze::Schema;

/// A registered user of the system.
#[derive(Schema)]
#[allow(dead_code)]
struct User {
    /// The user's id.
    id: i64,
    /// The user's display name.
    name: String,
}

#[test]
fn struct_with_field_and_top_descriptions() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    User:
      type: object
      description: A registered user of the system.
      required:
        - id
        - name
      properties:
        id:
          type: integer
          description: "The user's id."
          format: int64
        name:
          type: string
          description: "The user's display name."
    "#);
}
