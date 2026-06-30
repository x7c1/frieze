//! Schema name composition and end-to-end registration for a
//! single-argument generic struct instantiated with a primitive scalar:
//! `Container<i64>` produces `Int64_Container`, and the inner `value: T`
//! field renders inline as the scalar shape
//! (`{type: integer, format: int64}`) instead of as a `$ref` to a
//! non-existent `Int64` component.
//!
//! Generic derive output cannot know at expansion time whether `T` is a
//! primitive, so it emits `PropertyType::Reference(<T as Schema>::name())`
//! unconditionally. The boundary conversion in `frieze-usecase` looks
//! the reference name up against the primitive-name table and inlines
//! the scalar shape at the leaf position. The OAS document therefore
//! contains no `components/schemas/Int64` entry and no dangling `$ref`.
//!
//! Identical output under `oas-3-0` and `oas-3-1`.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[test]
fn container_i64_name_uses_suffix_form() {
    assert_eq!(<Container<i64> as Schema>::name(), "Int64_Container");
}

#[test]
fn container_string_name_uses_suffix_form() {
    assert_eq!(<Container<String> as Schema>::name(), "String_Container");
}

#[test]
fn container_bool_name_uses_suffix_form() {
    assert_eq!(<Container<bool> as Schema>::name(), "Boolean_Container");
}

#[test]
fn container_i64_inlines_primitive_value() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Container<i64>>()
        .build()
        .expect("schemas build succeeds without a registered `Int64`");

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
    ");
}

#[test]
fn container_string_inlines_primitive_value() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Container<String>>()
        .build()
        .expect("schemas build succeeds without a registered `String`");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        String_Container:
          type: object
          required:
          - value
          properties:
            value:
              type: string
    ");
}

#[test]
fn container_bool_inlines_primitive_value() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Container<bool>>()
        .build()
        .expect("schemas build succeeds without a registered `Boolean`");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Boolean_Container:
          type: object
          required:
          - value
          properties:
            value:
              type: boolean
    ");
}
