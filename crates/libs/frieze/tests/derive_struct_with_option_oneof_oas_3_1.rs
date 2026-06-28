//! `Option<E>` where `E` is an internally-tagged enum uses the same
//! OAS 3.1 nullable-reference wrap as `Option<U>` over a struct or a
//! string-enum: `{oneOf: [{$ref: E}, {type: "null"}]}`.

#![cfg(feature = "oas-3-1")]

use frieze::Schema;
use serde::{Deserialize, Serialize};

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
    event: Option<Event>,
}

#[test]
fn option_oneof_field_wraps_with_oneof_null() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Audit>()
        .add::<Event>()
        .add::<LoginData>()
        .add::<LogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Audit:
      type: object
      required:
        - id
        - event
      properties:
        id:
          type: integer
          format: int64
        event:
          oneOf:
            - $ref: "#/components/schemas/Event"
            - type: "null"
    Event:
      oneOf:
        - allOf:
            - $ref: "#/components/schemas/LoginData"
            - type: object
              required:
                - kind
              properties:
                kind:
                  type: string
                  enum:
                    - Login
        - allOf:
            - $ref: "#/components/schemas/LogoutData"
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
    "##);
}
