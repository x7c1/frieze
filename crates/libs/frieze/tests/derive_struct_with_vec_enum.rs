//! A `Vec<Status>` field renders as `type: array, items: {$ref}`. The
//! shape is identical under both `oas-3-0` and `oas-3-1` (no
//! nullability is involved).

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    statuses: Vec<Status>,
}

#[test]
fn vec_of_enum_field_renders_as_array_of_refs() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Status:
          type: string
          enum:
          - Active
          - Inactive
        User:
          type: object
          required:
          - statuses
          properties:
            statuses:
              type: array
              items:
                $ref: '#/components/schemas/Status'
    ");
}
