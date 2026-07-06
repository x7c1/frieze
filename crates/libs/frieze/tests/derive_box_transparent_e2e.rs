//! `Box<T>` is transparent end-to-end: a field of type `Box<User>` is
//! emitted as `$ref: User` exactly like a bare `User` field, and no
//! synthetic `User_Box` entry appears under `#/components/schemas`.
//!
//! This is the end-to-end upgrade of the trait-level assertions in
//! `derive_box_transparent.rs`. The field-type integration (lifting the
//! generic-argument rejection in the macro) is what makes the `Box<User>`
//! parse path work.
//!
//! Identical output under OAS 3.0 and 3.1.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Owner {
    boxed: Box<User>,
}

#[test]
fn box_field_renders_as_inner_ref_with_no_box_entry() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Owner>()
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
        Owner:
          type: object
          required:
          - boxed
          properties:
            boxed:
              $ref: '#/components/schemas/User'
        User:
          type: object
          required:
          - id
          - name
          properties:
            id:
              type: integer
              format: int64
            name:
              type: string
    ");
}
