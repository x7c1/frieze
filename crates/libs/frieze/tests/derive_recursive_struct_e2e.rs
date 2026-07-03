//! Recursive struct end-to-end: `struct Tree { value: i64, children:
//! Vec<Box<Tree>> }` self-references via `$ref: Tree`. The transparency
//! of `Box<T>` (see `wrapper_impls`) makes this work: `<Box<Tree> as
//! Schema>::name() == "Tree"`, so the array `items` resolve to the same
//! `Tree` entry and the transitive-closure walker terminates.
//!
//! This is the end-to-end upgrade of the trait-level assertions in
//! `derive_recursive_struct.rs`; the field-type integration makes the
//! `Vec<Box<Tree>>` parse path work.
//!
//! Identical output under OAS 3.0 and 3.1.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
// `Vec<Box<Tree>>` is the canonical recursive-Rust pattern this test is
// exercising end-to-end; the `vec_box` lint's preferred `Vec<Tree>` form
// loses the indirection point that recursive type definitions need.
#[allow(clippy::vec_box)]
struct Tree {
    value: i64,
    children: Vec<Box<Tree>>,
}

#[test]
fn recursive_struct_emits_self_referential_ref() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Tree>()
        .build()
        .expect("schemas build should succeed for a self-referential type");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
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
          - children
          properties:
            value:
              type: integer
              format: int64
            children:
              type: array
              items:
                $ref: '#/components/schemas/Tree'
    ");
}
