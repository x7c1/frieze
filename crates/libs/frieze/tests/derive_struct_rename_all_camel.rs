//! Container-level `#[serde(rename_all = "camelCase")]` rewrites every
//! struct field name to camelCase on the wire — both in the
//! `properties` map and in the `required` array.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct User {
    user_id: i64,
    display_name: String,
}

#[test]
fn struct_rename_all_camel_case_renames_every_field() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
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
          - userId
          - displayName
          properties:
            userId:
              type: integer
              format: int64
            displayName:
              type: string
    ");
}
