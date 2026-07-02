//! Nested generic-arg composition: `Container<Container<i64>>` produces
//! `Int64_Container_Container` — each layer's `name()` recursively
//! composes the layer-inner's name with its own base, yielding a flat
//! concatenation without doubled separators.
//!
//! End-to-end, the inner `Container<i64>` registers as
//! `Int64_Container` (with the primitive inlined at its leaf), and the
//! outer `Container<Container<i64>>` carries a regular `$ref` to that
//! entry. Two layers of registration suffice; no `Int64` component
//! entry is needed.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[test]
fn container_of_container_of_i64_name_collapses_recursively() {
    assert_eq!(
        <Container<Container<i64>> as Schema>::name(),
        "Int64_Container_Container"
    );
}

#[test]
fn container_of_container_of_string_name_collapses_recursively() {
    assert_eq!(
        <Container<Container<String>> as Schema>::name(),
        "String_Container_Container"
    );
}

#[test]
fn container_of_container_of_i64_registers_two_layers() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Container<Container<i64>>>()
        .add::<Container<i64>>()
        .build()
        .expect("two-layer registration resolves the inner reference and inlines the primitive");

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
        Int64_Container_Container:
          type: object
          required:
          - value
          properties:
            value:
              $ref: '#/components/schemas/Int64_Container'
    ");
}
