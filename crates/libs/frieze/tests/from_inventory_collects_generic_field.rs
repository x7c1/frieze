//! End-to-end: `from_inventory()` *alone* auto-collects a generic
//! instantiation (`Page<Bar>`) reached through a non-generic root's
//! field, with no explicit `add::<T>()` calls.
//!
//! This is the integration test for the central claim of the
//! auto-collection mechanism: "derive a few types and call
//! `from_inventory().build()`; generic instances on field positions
//! are pulled in automatically."
//!
//! The inventory channel submits only the two non-generic types
//! (`Foo` and `Bar`); `Page<T>` is generic and intentionally not
//! submitted (Rust's `static` cannot hold generic types). At runtime,
//! `from_inventory()` invokes `Foo::register_into`, which emits the
//! syntactic call `<Page<Bar> as Schema>::register_into(builder)`.
//! `Page<Bar>` is monomorphized at compile time, so the call resolves
//! to `Page<Bar>`'s own `register_into` body, registering itself as
//! `Bar_Page` and recursing into `Bar` (which the idempotent guard
//! collapses with the inventory-submitted entry).
//!
//! This test is only meaningful with `--features inventory`; the file
//! is gated accordingly.

#![cfg(feature = "inventory")]

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
fn from_inventory_walks_into_generic_field() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory + transitive walk produces a closed schemas set");

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
