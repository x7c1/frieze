//! Variant docs lift into the enum's description as `- <name>: <doc>`
//! bullets, and the `<name>` reflects the **wire name** — including
//! when the wire name comes from a variant-level `rename` rather than
//! from `rename_all`.

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
    /// The entity is gone.
    #[serde(rename = "gone")]
    InactiveSince,
}

#[test]
fn variant_bullet_names_use_individual_rename_when_present() {
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
            - gone: The entity is gone.
          enum:
          - active
          - gone
    ");
}
