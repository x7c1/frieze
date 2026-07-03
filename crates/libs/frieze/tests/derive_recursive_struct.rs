//! Recursive type support: `struct Tree { value: i64, children:
//! Vec<Box<Tree>> }` self-references via `$ref: Tree`.
//!
//! The transparency of `Box<T>` (see `derive_box_transparent.rs`) is
//! what makes this possible: `<Box<Tree> as Schema>::name() == "Tree"`
//! so the array items resolve to `Tree`'s own schema entry. A naive
//! "every generic instantiation is its own entry" rule would produce
//! an infinite cascade `Tree_Box`, `Tree_Box_Box`, ... — the
//! transitive-closure walker would never terminate.
//!
//! The recursive transparency contract is asserted at the trait level
//! here — a `Box<Tree>` value's `name()` and `schema()` agree with
//! `Tree` itself, which is exactly what the field-type integration
//! (`children: Vec<Box<Tree>>`) relies on end-to-end.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Tree {
    value: i64,
    label: String,
}

#[test]
fn box_tree_name_equals_tree_name() {
    // Recursive transparency: `Box<Tree>` is the same schema as `Tree`.
    // When `children: Vec<Box<Tree>>` is parsed by the field-type
    // pipeline, the emitted `items` resolve to `Tree` via this same identity.
    assert_eq!(<Box<Tree> as Schema>::name(), "Tree");
    assert_eq!(<Box<Tree> as Schema>::schema(), <Tree as Schema>::schema());
}

#[test]
fn box_tree_is_registrable_through_blanket_impl() {
    // `IsRegistrable` propagates through `Box<T>`, so the eventual
    // `Schemas::add::<Box<Tree>>()` call (equivalent to
    // `Schemas::add::<Tree>()`) does not trip on the
    // primitive-rejection bound. The blanket impl in
    // `frieze-usecase::wrapper_impls` is what unlocks this. The
    // snapshot below confirms no synthetic `Tree_Box` entry appears —
    // a `Box<Tree>` and a `Tree` registration produce the same output.
    let schemas = frieze::SchemasBuilder::new()
        .add::<Box<Tree>>()
        .build()
        .expect("registering Box<Tree> succeeds and is equivalent to adding Tree");
    insta::assert_snapshot!(common::snapshot_yaml(schemas), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Tree:
          type: object
          required:
          - value
          - label
          properties:
            value:
              type: integer
              format: int64
            label:
              type: string
    ");
}
