//! When both `#[serde(rename_all = "...")]` (on the container) and
//! `#[serde(rename = "literal")]` (on an individual field) apply, the
//! individual `rename` wins — matching serde's own precedence.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct User {
    // Container rule would produce `userId`; the individual rename
    // pins it to `external_id` instead.
    #[serde(rename = "external_id")]
    user_id: i64,
    // No individual rename — the camelCase rule applies.
    display_name: String,
}

#[test]
fn individual_field_rename_takes_precedence_over_rename_all() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        User:
          type: object
          required:
          - external_id
          - displayName
          properties:
            external_id:
              type: integer
              format: int64
            displayName:
              type: string
    ");
}
