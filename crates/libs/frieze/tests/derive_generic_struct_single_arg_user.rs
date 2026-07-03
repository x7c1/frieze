//! Schema name composition and end-to-end registration for a single-
//! argument generic struct instantiated with a user-defined type:
//! `Page<User>` produces `User_Page` and registers cleanly alongside
//! `User`.
//!
//! Identical output under `oas-3-0` and `oas-3-1`.

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
struct Page<T> {
    items: Vec<T>,
    total: i64,
}

#[test]
fn page_user_name_uses_suffix_form() {
    assert_eq!(<Page<User> as Schema>::name(), "User_Page");
}

#[test]
fn page_user_registers_alongside_user() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Page<User>>()
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
        User_Page:
          type: object
          required:
          - items
          - total
          properties:
            items:
              type: array
              items:
                $ref: '#/components/schemas/User'
            total:
              type: integer
              format: int64
    ");
}
