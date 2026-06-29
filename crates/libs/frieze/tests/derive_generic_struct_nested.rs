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
    let s: frieze::Schemas = frieze::schemas()
        .add::<Container<Container<i64>>>()
        .add::<Container<i64>>()
        .build()
        .expect("two-layer registration resolves the inner reference and inlines the primitive");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
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
          $ref: "#/components/schemas/Int64_Container"
    "##);
}
