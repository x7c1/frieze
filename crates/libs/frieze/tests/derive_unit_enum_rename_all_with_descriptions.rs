//! When `#[serde(rename_all = ...)]` rewrites the variant names, the
//! bullet list in the composed description uses the **OAS output name**
//! (post-rename) so the bullets line up 1:1 with the `enum` array.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

/// Lifecycle state of an entity.
#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Status {
    /// The entity is currently active.
    Active,
    /// The entity is no longer active.
    InactiveSince,
}

#[test]
fn variant_bullet_names_use_post_rename_form() {
    let s: frieze::Schemas = frieze::schemas()
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

            - active: The entity is currently active.
            - inactive_since: The entity is no longer active.
          enum:
          - active
          - inactive_since
    ");
}
