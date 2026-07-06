//! `Option<Status>` (serde default — no `skip_serializing_if`) produces
//! the required + nullable shape (`Option<U>` serde default in
//! `docs/field-shapes.md`, with `U` an enum schema here). Under OAS 3.0,
//! a nullable reference to an enum schema is expressed with `allOf` +
//! `nullable: true` — the same wrap used for nullable nested-struct
//! references.

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
    status: Option<Status>,
}

#[test]
fn option_enum_field_renders_as_nullable_ref_under_oas_3_0() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_0(s), @"
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
          - status
          properties:
            status:
              allOf:
              - $ref: '#/components/schemas/Status'
              nullable: true
    ");
}
