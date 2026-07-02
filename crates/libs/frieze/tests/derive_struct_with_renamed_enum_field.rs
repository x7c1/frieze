//! `$ref` composition: when a struct field's type is a `Schema`-deriving
//! enum that itself uses `rename_all`, the renamed variants live inside
//! the referenced enum schema, while the struct's own field is renamed
//! independently. The two `rename` decisions do not interact.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Lifecycle {
    Active,
    InactiveSince,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    #[serde(rename = "lifecycle")]
    user_lifecycle: Lifecycle,
}

#[test]
fn renamed_field_keeps_referenced_enum_rename_all_intact() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Lifecycle>()
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
        Lifecycle:
          type: string
          enum:
          - active
          - inactive_since
        User:
          type: object
          required:
          - lifecycle
          properties:
            lifecycle:
              $ref: '#/components/schemas/Lifecycle'
    ");
}
