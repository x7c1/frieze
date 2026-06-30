//! `Vec<Page<User>>` field: the generic-instantiation `Page<User>` is
//! treated as a struct reference inside the array's `items`.
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

#[derive(Schema)]
#[allow(dead_code)]
struct Listings {
    pages: Vec<Page<User>>,
}

#[test]
fn vec_generic_renders_as_array_of_ref() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Listings>()
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
        Listings:
          type: object
          required:
          - pages
          properties:
            pages:
              type: array
              items:
                $ref: '#/components/schemas/User_Page'
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
