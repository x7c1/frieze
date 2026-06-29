//! An internally-tagged enum (`#[serde(tag = "kind")]`) whose every
//! variant is a newtype of a `Schema`-implementing struct derives a
//! `oneOf` schema. Each arm is `allOf: [{$ref}, {synthetic tag-property
//! object}]`, and the enclosing schema carries
//! `discriminator: {propertyName: <tag>}` with no `mapping` block.
//!
//! The output is identical under both `oas-3-0` and `oas-3-1`.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct LoginData {
    user_id: i64,
    session: String,
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

#[test]
fn renders_as_one_of_with_internal_tag_allof() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Event>()
        .add::<LoginData>()
        .add::<LogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
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
        - session
      properties:
        user_id:
          type: integer
          format: int64
        session:
          type: string
    LogoutData:
      type: object
      required:
        - reason
      properties:
        reason:
          type: string
    "##);
}
