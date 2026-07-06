//! `Maybe<E>` where `E` is an internally-tagged enum maps to the
//! optional + nullable shape (`Maybe<T>` in `docs/field-shapes.md`):
//! the field is dropped from `required`, and the reference to `E` is
//! rendered through the OAS-3.0 nullable-ref wrap
//! (`{allOf: [{$ref: E}], nullable: true}`) — the same composition used
//! by `Maybe<U>` over a struct and by `Option<E>` over the same enum
//! shape. Guards against regressions in the shared `Maybe` × `$ref`
//! composition path when applied to `OneOf` references.

use frieze::Schema;
use frieze_model::Maybe;
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
    id: i64,
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    event: Maybe<Event>,
}

#[test]
fn maybe_oneof_field_wraps_with_allof_nullable_and_drops_required() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Audit>()
        .add::<Event>()
        .add::<LoginData>()
        .add::<LogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_0(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Audit:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
            event:
              allOf:
              - $ref: '#/components/schemas/Event'
              nullable: true
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
