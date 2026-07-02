//! End-to-end: a single `add::<Foo>()` auto-collects a generic
//! instantiation (`Page<Bar>`) and its inner argument (`Bar`) through
//! the transitive `Schema::register_into` walk.
//!
//! Why this matters: `inventory` cannot carry generic types in a
//! `static` (Rust's language limitation), but the derived
//! `register_into` is a regular monomorphic function — so when `Foo`'s
//! derive emits `<Page<Bar> as Schema>::register_into(builder)` it is
//! `Page<Bar>`'s own derived body (with `T = Bar`) that runs at
//! runtime, registers itself as `Bar_Page`, and recurses into `Bar`.
//! The end result is that adding only the non-generic root pulls in
//! every monomorphic instance reachable through the field graph.
//!
//! Identical output under `oas-3-0` and `oas-3-1`.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Bar {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Page<T> {
    items: Vec<T>,
    total: u64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Foo {
    page: Page<Bar>,
}

#[test]
fn single_add_auto_collects_generic_instance() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Foo>()
        .build()
        .expect("transitive `register_into` collects `Page<Bar>` and `Bar` automatically");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Bar:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        Bar_Page:
          type: object
          required:
          - items
          - total
          properties:
            items:
              type: array
              items:
                $ref: '#/components/schemas/Bar'
            total:
              type: integer
              format: int64
              minimum: 0
        Foo:
          type: object
          required:
          - page
          properties:
            page:
              $ref: '#/components/schemas/Bar_Page'
    ");
}
