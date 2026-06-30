//! End-to-end: `from_inventory()` + `add::<T>()` chain to cover types
//! the inventory channel cannot reach.
//!
//! Generic instantiations like `Page<Bar>` are NOT in the inventory
//! (Rust's `static` cannot hold generic types, so the derive on
//! `Page<T>` does not emit a submission). When no non-generic root's
//! field references `Page<Bar>`, the only way to register it is an
//! explicit `add::<Page<Bar>>()`.
//!
//! Here `Standalone` is in inventory (collected via `from_inventory`)
//! and `Page<Bar>` is not — yet `add::<Page<Bar>>()` chained after
//! `from_inventory()` brings it (and its inner `Bar`) into the same
//! schemas set. `Bar` arrives via two channels (the inventory
//! submission for its non-generic derive AND the transitive walk from
//! `Page<Bar>`); the idempotent guard collapses them into one entry.
//!
//! This test is only meaningful with `--features inventory`; the file
//! is gated accordingly.

#![cfg(feature = "inventory")]

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Standalone {
    name: String,
}

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

#[test]
fn from_inventory_composes_with_explicit_generic_root() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        // `Page<Bar>` cannot ride the inventory channel (`Page<T>` is
        // generic) and no inventory-submitted type's field reaches it.
        // The explicit `add` is the only way it ends up in the
        // schemas.
        .add::<Page<Bar>>()
        .build()
        .expect("inventory + add chain produces a closed schemas set");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
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
            $ref: "#/components/schemas/Bar"
        total:
          type: integer
          format: int64
          minimum: 0
    Standalone:
      type: object
      required:
        - name
      properties:
        name:
          type: string
    "##);
}
