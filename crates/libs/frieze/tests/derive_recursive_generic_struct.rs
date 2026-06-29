//! Recursive + generic combined: `struct Node<T> { value: T, next:
//! Option<Box<Node<T>>> }` instantiated as `Node<User>` self-references
//! via `$ref: User_Node`. The `Box<T>` transparency collapses
//! `Box<Node<User>>` to the same `User_Node` entry, so the
//! transitive-closure walker terminates.
//!
//! Identical output under `oas-3-0` and `oas-3-1` (the inner `Option`
//! over a reference renders differently between versions, so we cover
//! both feature flags below).

use frieze::Schema;

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

#[cfg(feature = "oas-3-0")]
#[test]
fn node_user_self_references_under_oas_3_0() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<User>>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for a recursive generic type");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
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
          $ref: "#/components/schemas/User"
        next:
          allOf:
            - $ref: "#/components/schemas/User_Node"
          nullable: true
    "##);
}

#[cfg(feature = "oas-3-1")]
#[test]
fn node_user_self_references_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Node<User>>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for a recursive generic type");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
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
          $ref: "#/components/schemas/User"
        next:
          oneOf:
            - $ref: "#/components/schemas/User_Node"
            - type: "null"
    "##);
}
