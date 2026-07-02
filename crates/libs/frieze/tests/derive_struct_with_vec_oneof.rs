//! `Vec<E>` where `E` is an internally-tagged enum is emitted as
//! `{type: array, items: {$ref: E}}`, reusing the same array-of-ref
//! shape that nested struct and string-enum references use.
//!
//! The output is identical under both `oas-3-0` and `oas-3-1`.

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
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Event {
    Login(LoginData),
    Logout(LogoutData),
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Audit {
    events: Vec<Event>,
}

#[test]
fn vec_of_oneof_field_renders_as_array_of_ref() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Audit>()
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
        Audit:
          type: object
          required:
          - events
          properties:
            events:
              type: array
              items:
                $ref: '#/components/schemas/Event'
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
                  - Login
          - allOf:
            - $ref: '#/components/schemas/LogoutData'
            - type: object
              required:
              - kind
              properties:
                kind:
                  type: string
                  enum:
                  - Logout
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
