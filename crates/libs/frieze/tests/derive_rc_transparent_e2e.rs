//! `Rc<T>` is transparent end-to-end: a field of type `Rc<User>` is
//! emitted as `$ref: User`, no synthetic entry is generated. Parallel to
//! `derive_box_transparent_e2e.rs`.

use std::rc::Rc;

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
    shared: Rc<User>,
}

#[test]
fn rc_field_renders_as_inner_ref_with_no_rc_entry() {
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
          - shared
          properties:
            shared:
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
