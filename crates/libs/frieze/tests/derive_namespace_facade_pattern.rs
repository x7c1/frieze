//! End-to-end: a regular `mod foo; pub use foo::Foo;` facade pattern
//! produces the bare OAS key `Foo` — no namespace prefix.
//!
//! This is the back-stop test for the central promise:
//! when no `#[frieze(namespace)]` attribute is in scope, the
//! `compose_schema_name` helper is the identity and the OAS keys are
//! byte-identical to the pre-namespace derive output. Existing snapshot
//! tests already cover the "no namespace anywhere" case for many
//! shapes; this test specifically anchors the facade-pattern
//! interpretation — `mod foo;` (no attribute) is *implementation
//! detail* the user has chosen not to surface, and the OAS reflects
//! that.
//!
//! Gated on `--features inventory` so it exercises the namespace
//! lookup path (which short-circuits when no namespaces are
//! registered) rather than the no-inventory build path tested
//! separately in `derive_namespace_disabled`.

#![cfg(feature = "inventory")]

mod common;
mod inner {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct Foo {
        pub id: i64,
    }
}

#[allow(unused_imports)]
pub use inner::Foo;

#[test]
fn facade_pattern_keeps_bare_oas_key() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    // No `#[frieze(namespace)]` reaches this binary, so the namespace
    // set is empty and `compose_schema_name(_, "Foo")` returns
    // `"Foo"` unchanged. The `mod inner` lives at module path
    // `derive_namespace_facade_pattern::inner`, but `inner` is not a
    // declared namespace, so it is dropped during the prefix walk.
    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Foo:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
    ");
}
