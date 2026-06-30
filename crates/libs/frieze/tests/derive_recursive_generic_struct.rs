//! Recursive + generic combined: `struct Node<T> { value: T, next:
//! Option<Box<Node<T>>> }` instantiated as `Node<User>` self-references
//! via `$ref: User_Node`. The `Box<T>` transparency collapses
//! `Box<Node<User>>` to the same `User_Node` entry, so the
//! transitive-closure walker terminates.
//!
//! `Node<i64>` exercises the same self-reference path with a primitive
//! value: the recursive `next` pointer still resolves to
//! `$ref: Int64_Node`, while the `value` field inlines as
//! `{type: integer, format: int64}` rather than emitting a dangling
//! `$ref` to `Int64`.
//!
//! Inner `Option`-over-reference renders differently between OAS 3.0
//! and 3.1, so each version has its own snapshot below.

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
struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

#[test]
fn node_user_name_uses_suffix_form() {
    assert_eq!(<Node<User> as Schema>::name(), "User_Node");
}

#[test]
fn node_i64_name_uses_suffix_form() {
    assert_eq!(<Node<i64> as Schema>::name(), "Int64_Node");
}

#[cfg(feature = "oas-3-0")]
#[test]
fn node_user_self_references_under_oas_3_0() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<User>>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for a recursive generic type");

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
        User_Node:
          type: object
          required:
          - value
          - next
          properties:
            value:
              $ref: '#/components/schemas/User'
            next:
              allOf:
              - $ref: '#/components/schemas/User_Node'
              nullable: true
    ");
}

#[cfg(feature = "oas-3-1")]
#[test]
fn node_user_self_references_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<User>>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for a recursive generic type");

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
        User_Node:
          type: object
          required:
          - value
          - next
          properties:
            value:
              $ref: '#/components/schemas/User'
            next:
              oneOf:
              - $ref: '#/components/schemas/User_Node'
              - type: 'null'
    ");
}

#[cfg(feature = "oas-3-0")]
#[test]
fn node_i64_self_references_with_inline_primitive_under_oas_3_0() {
    // The `value: T` field with `T = i64` inlines as the primitive
    // scalar shape; the recursive `next` still threads through
    // `$ref: Int64_Node`, exercising the self-reference path under a
    // primitive instantiation. No `Int64` entry is needed (or emitted)
    // for build to succeed.
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<i64>>()
        .build()
        .expect("schemas build should succeed for a recursive generic over a primitive");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Int64_Node:
          type: object
          required:
          - value
          - next
          properties:
            value:
              type: integer
              format: int64
            next:
              allOf:
              - $ref: '#/components/schemas/Int64_Node'
              nullable: true
    ");
}

#[cfg(feature = "oas-3-1")]
#[test]
fn node_i64_self_references_with_inline_primitive_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<i64>>()
        .build()
        .expect("schemas build should succeed for a recursive generic over a primitive");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Int64_Node:
          type: object
          required:
          - value
          - next
          properties:
            value:
              type: integer
              format: int64
            next:
              oneOf:
              - $ref: '#/components/schemas/Int64_Node'
              - type: 'null'
    ");
}
