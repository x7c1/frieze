//! `Maybe<Page<User>>` field: the generic-instantiation `Page<User>` is
//! treated as a struct reference, and the surrounding `Maybe<T>` wraps
//! it as `optional + nullable` under the OAS-3.1 nullable-reference
//! shape (`oneOf: [$ref, {type: "null"}]`).

#![cfg(feature = "oas-3-1")]

use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Page<T> {
    items: Vec<T>,
    total: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Listing {
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    page: Maybe<Page<User>>,
}

#[test]
fn maybe_generic_renders_as_optional_nullable_ref_under_oas_3_1() {
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
          properties:
            page:
              oneOf:
              - $ref: '#/components/schemas/User_Page'
              - type: 'null'
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
