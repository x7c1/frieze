//! `Maybe<T>` under OAS 3.1: the field is dropped from `required` and the
//! inner schema's `type` becomes a 2-element sequence `[<base>, "null"]`.

#![cfg(feature = "oas-3-1")]

use frieze::Schema;
use frieze_model::Maybe;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize, Debug, PartialEq)]
struct Profile {
    id: i64,
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    avatar_url: Maybe<String>,
}

#[test]
fn maybe_field_renders_optional_and_type_null_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Profile:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
            avatar_url:
              type:
              - string
              - 'null'
    ");
}
