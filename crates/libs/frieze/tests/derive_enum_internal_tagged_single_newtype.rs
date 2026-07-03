//! The minimal 1-variant internally-tagged enum still derives the full
//! `oneOf + discriminator` shape; `oneOf` may carry a single arm, and
//! the discriminator block emits `propertyName` only (no `mapping`).
//!
//! The output is identical under both OAS 3.0 and 3.1.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct OnlyData {
    value: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Single {
    Only(OnlyData),
}

#[test]
fn single_variant_internal_tagged_still_emits_one_of() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Single>()
        .add::<OnlyData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        OnlyData:
          type: object
          required:
          - value
          properties:
            value:
              type: integer
              format: int64
        Single:
          oneOf:
          - allOf:
            - $ref: '#/components/schemas/OnlyData'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Only
          discriminator:
            propertyName: kind
    ");
}
