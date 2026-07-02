//! A generic struct (`Container<T>`) used as the inner of an
//! internally-tagged enum variant: the per-variant `IsStructSchema`
//! bound check accepts the generic instantiation because
//! `Container<T>: IsStructSchema` holds whenever `T: Schema` holds.
//!
//! The wire shape is identical under `oas-3-0` and `oas-3-1`.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Event {
    Held(Container<User>),
}

#[test]
fn generic_inner_in_internal_tagged_enum() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Event>()
        .add::<Container<User>>()
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
        Event:
          oneOf:
          - allOf:
            - $ref: '#/components/schemas/User_Container'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Held
          discriminator:
            propertyName: kind
        User:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        User_Container:
          type: object
          required:
          - value
          properties:
            value:
              $ref: '#/components/schemas/User'
    ");
}
