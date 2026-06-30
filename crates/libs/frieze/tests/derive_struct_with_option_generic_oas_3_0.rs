//! `Option<Page<User>>` field: the generic-instantiation `Page<User>`
//! is treated as a struct reference, and the surrounding `Option<T>`
//! wraps that reference in the OAS-3.0 nullable-reference shape
//! (`allOf: [$ref], nullable: true`).

#![cfg(feature = "oas-3-0")]

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
struct Listing {
    page: Option<Page<User>>,
}

#[test]
fn option_generic_renders_as_nullable_ref_under_oas_3_0() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Listing>()
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
        Listing:
          type: object
          required:
          - page
          properties:
            page:
              allOf:
              - $ref: '#/components/schemas/User_Page'
              nullable: true
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
