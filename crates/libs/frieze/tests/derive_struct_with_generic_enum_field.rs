//! A struct field whose type is a generic-enum instantiation
//! (`Event<i64, String>`) emits a `$ref` to the enum's composed schema
//! name (`Int64_String_Event`). Verifies the field-side recognition of
//! a generic-typed path delivers the suffix-form name correctly through
//! `<#ty as Schema>::name()`.
//!
//! The shape is identical under `oas-3-0` and `oas-3-1`.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Event<T, U> {
    Held(Container<T>),
    Lost(Container<U>),
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Log {
    event: Event<i64, String>,
}

#[test]
fn log_field_refs_composed_event_name() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Log>()
        .add::<Event<i64, String>>()
        .add::<Container<i64>>()
        .add::<Container<String>>()
        .build()
        .expect(
            "schemas build resolves the Event<i64, String> reference and its transitive closure",
        );

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Int64_Container:
          type: object
          required:
          - value
          properties:
            value:
              type: integer
              format: int64
        Int64_String_Event:
          oneOf:
          - allOf:
            - $ref: '#/components/schemas/Int64_Container'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Held
          - allOf:
            - $ref: '#/components/schemas/String_Container'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Lost
          discriminator:
            propertyName: kind
        Log:
          type: object
          required:
          - event
          properties:
            event:
              $ref: '#/components/schemas/Int64_String_Event'
        String_Container:
          type: object
          required:
          - value
          properties:
            value:
              type: string
    ");
}
