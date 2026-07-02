//! Enum-level doc + only some variants with their own doc:
//!
//! - bullet list appears, with rows for documented variants only.
//! - the `enum` array still contains every variant in declaration order.

use frieze::Schema;

mod common;

/// Lifecycle state of an entity.
#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    /// The entity is currently active.
    Active,
    Pending,
    /// The entity is no longer active.
    Inactive,
}

#[test]
fn missing_variant_docs_are_omitted_from_the_bullet_list() {
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
          description: |-
            Lifecycle state of an entity.

            - Active: The entity is currently active.
            - Inactive: The entity is no longer active.
          enum:
          - Active
          - Pending
          - Inactive
    ");
}
