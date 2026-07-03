//! A non-nullable nested reference with a `///` doc on the field. Under
//! OAS 3.0 a `$ref` schema's siblings are ignored on the wire, so the
//! emitter wraps the reference in `allOf` and places the description on
//! the outer schema (the `allOf` wrapper).

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    /// The user account this profile belongs to.
    user: User,
}

#[test]
fn ref_field_with_description_wraps_in_all_of_under_oas_3_0() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Profile:
          type: object
          required:
          - user
          properties:
            user:
              description: The user account this profile belongs to.
              allOf:
              - $ref: '#/components/schemas/User'
        User:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
    ");
}
