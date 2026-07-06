//! Individual `#[serde(rename = "literal")]` on a variant of an
//! internally-tagged enum overrides the container's `rename_all` (and
//! the Rust identifier) for that variant's tag value, matching the
//! precedence rule used everywhere else in frieze.
//!
//! The output is identical under both OAS 3.0 and 3.1.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct LoginData {
    user_id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct LogoutData {
    reason: String,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)]
enum Event {
    #[serde(rename = "AUTH_LOGIN")]
    Login(LoginData),
    Logout(LogoutData),
}

#[test]
fn individual_rename_overrides_rename_all_for_tag_value() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Event>()
        .add::<LoginData>()
        .add::<LogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Event:
          oneOf:
          - allOf:
            - $ref: '#/components/schemas/LoginData'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - AUTH_LOGIN
          - allOf:
            - $ref: '#/components/schemas/LogoutData'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - logout
          discriminator:
            propertyName: kind
        LoginData:
          type: object
          required:
          - user_id
          properties:
            user_id:
              type: integer
              format: int64
        LogoutData:
          type: object
          required:
          - reason
          properties:
            reason:
              type: string
    ");
}
