//! End-to-end: a single `add::<Foo>()` produces the full transitive
//! closure for a simple two-level struct graph. The derive emits a
//! `Schema::register_into` body that walks each field type's
//! `register_into`, so the nested `User` is registered automatically
//! through the field walk on `Foo.user`.
//!
//! This is the foundational PR-1 case — the auto-collection feature in
//! its smallest form. Identical output under `oas-3-0` and `oas-3-1`.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Foo {
    user: User,
}

#[test]
fn single_add_auto_collects_transitive_struct() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Foo>()
        .build()
        .expect("transitive `register_into` collects the nested `User` automatically");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Foo:
      type: object
      required:
        - user
      properties:
        user:
          $ref: "#/components/schemas/User"
    User:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}
