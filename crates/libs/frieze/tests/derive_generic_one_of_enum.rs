//! Generic internal-tagged enum with two type parameters and two
//! newtype-of-generic-struct variants. End-to-end snapshot of
//! `Event<i64, String>` instantiated with primitive arguments verifies:
//!
//! - the type-parameter bound (`T: Schema`, `U: Schema`) is propagated
//!   onto the emitted `impl Schema for Event<T, U>` block;
//! - the suffix-form composed schema name
//!   (`Int64_String_Event`) is computed at monomorphisation time;
//! - per-variant `IsStructSchema` bound checks accept generic-struct
//!   inners (`Container<T>`, `Container<U>`) because
//!   `Container<i64>: IsStructSchema` is derived from
//!   `Container<T>: IsStructSchema where T: Schema` plus
//!   `i64: Schema`;
//! - the inner generic structs (`Container<i64>` →
//!   `Int64_Container`, `Container<String>` → `String_Container`) each
//!   carry a primitive `value` field that the boundary conversion
//!   inlines at the leaf (no `$ref: Int64` / `$ref: String` dangle and
//!   no `components/schemas/Int64` / `components/schemas/String`
//!   entry).
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

#[test]
fn event_two_primitives_uses_suffix_form() {
    assert_eq!(<Event<i64, String> as Schema>::name(), "Int64_String_Event");
}

#[test]
fn event_argument_order_is_significant() {
    assert_eq!(<Event<String, i64> as Schema>::name(), "String_Int64_Event");
}

#[test]
fn event_renders_with_inlined_primitive_inners() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Event<i64, String>>()
        .add::<Container<i64>>()
        .add::<Container<String>>()
        .build()
        .expect("schemas build resolves the inner primitive references inline");

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
        String_Container:
          type: object
          required:
          - value
          properties:
            value:
              type: string
    ");
}
