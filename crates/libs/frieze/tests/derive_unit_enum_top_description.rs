//! Enum-level `///` doc with no per-variant docs renders only the
//! enum's own description — no bullet list is appended.

use frieze::Schema;

mod common;

/// Lifecycle state of an entity.
#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[test]
fn enum_top_doc_only_renders_without_variant_list() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Status:
          type: string
          description: Lifecycle state of an entity.
          enum:
          - Active
          - Inactive
    ");
}
