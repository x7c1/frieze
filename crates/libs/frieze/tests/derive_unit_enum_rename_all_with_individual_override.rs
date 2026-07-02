//! `#[serde(rename = "literal")]` on a single variant overrides the
//! container-level `rename_all` for that one entry. The remaining
//! variants still follow the container rule.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Status {
    Active,
    // Container rule would produce `inactive_since`; the individual
    // rename takes precedence.
    #[serde(rename = "gone")]
    InactiveSince,
}

#[test]
fn individual_variant_rename_takes_precedence_over_rename_all() {
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
          enum:
          - active
          - gone
    ");
}
