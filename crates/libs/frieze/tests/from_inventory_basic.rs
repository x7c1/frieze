//! End-to-end: `SchemasBuilder::new().from_inventory()` auto-collects
//! every non-generic `#[derive(Schema)]` type linked into the test
//! binary without any explicit `add::<T>()` calls.
//!
//! The derive emits an `inventory::submit!` site per non-generic input
//! (struct or enum); the `frieze` crate's `__private::inventory_submit!`
//! wrapper turns that into a real submission when the `inventory`
//! Cargo feature is on. `from_inventory()` then iterates every entry
//! and invokes its `register_fn`, which runs the derived
//! `Register::register_into` and walks each field type transitively.
//!
//! Test-binary scoping: each `tests/*.rs` file compiles as its own
//! binary, so the inventory iteration here only sees the types
//! declared in this file (`Root`, `Inner`). That isolation is what
//! makes the snapshot stable.
//!
//! This test is only meaningful with `--features inventory`; without
//! the feature the submission expands to a no-op and `from_inventory`
//! is not available. The whole file is gated under
//! `#[cfg(feature = "inventory")]` so the build still succeeds when
//! the feature is off.

#![cfg(feature = "inventory")]

use frieze::Schema;

mod common;

// `Inner` and `Root` deliberately omit `///` doc comments: doc comments
// would compose into the OAS `description` field and bloat the
// snapshot below. The rationale for the test types is kept here as
// `//` comments instead.
//
// `Inner` is a leaf struct referenced by `Root`. Its derive emits an
// inventory submission so `from_inventory()` picks it up as a root in
// addition to the field walk from `Root`.
//
// `Root.inner` references `Inner`; the derived `register_into` walks
// into `Inner::register_into`, where the idempotent guard collapses
// the second arrival into the existing entry.
#[derive(Schema)]
#[allow(dead_code)]
struct Inner {
    id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Root {
    inner: Inner,
}

#[test]
fn from_inventory_collects_every_derived_root() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Inner:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        Root:
          type: object
          required:
          - inner
          properties:
            inner:
              $ref: '#/components/schemas/Inner'
    ");
}
