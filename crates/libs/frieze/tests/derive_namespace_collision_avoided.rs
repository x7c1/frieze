//! End-to-end: two namespaces (`v1`, `v2`) each containing a
//! `struct User` co-exist as `v1.User` and `v2.User` without
//! collision.
//!
//! Exercises the motivating scenario for the namespace mechanism:
//! the bare-name collision that would
//! cause `Error::SchemaConflict` at build time is sidestepped by
//! the namespace mechanism. Per-binary inventory isolation means
//! both `User` instances live in this one test binary and the
//! build-time check on the unique OAS map below is the assertion.

#![cfg(feature = "inventory")]

mod common;
#[frieze::frieze(namespace)]
pub mod v1 {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct User {
        pub id: i64,
    }
}

#[frieze::frieze(namespace)]
pub mod v2 {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct User {
        pub uuid: String,
    }
}

#[test]
fn distinct_namespaces_resolve_same_bare_name() {
    // If the namespace mechanism failed to disambiguate, the
    // `from_inventory().build()` step would surface
    // `Error::SchemaConflict` because both `User` structs would
    // try to register under the bare key `User`. The successful
    // build (no `expect` panic) is the primary assertion; the
    // snapshot below is a secondary visualisation.
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .from_inventory()
        .build()
        .expect("namespace disambiguation prevents OAS key collision");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        v1.User:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
        v2.User:
          type: object
          required:
          - uuid
          properties:
            uuid:
              type: string
    ");
}
