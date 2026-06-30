use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Flag {
    enabled: bool,
    label: String,
}

#[test]
fn bool_field_renders_as_type_boolean() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Flag>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Flag:
          type: object
          required:
          - enabled
          - label
          properties:
            enabled:
              type: boolean
            label:
              type: string
    ");
}
