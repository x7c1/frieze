//! `#[serde(rename_all = "snake_case")]` on an internally-tagged enum
//! rewrites the tag value (the const string inside each arm's synthesized
//! tag property) using the variant `rename_all` rule. The variant
//! identifier (`UserLogin`) is invisible on the wire; the tag value is
//! `user_login`.
//!
//! The output is identical under both `oas-3-0` and `oas-3-1`.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct UserLoginData {
    user_id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct UserLogoutData {
    reason: String,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum Event {
    UserLogin(UserLoginData),
    UserLogout(UserLogoutData),
}

#[test]
fn rename_all_snake_case_rewrites_tag_values() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Event>()
        .add::<UserLoginData>()
        .add::<UserLogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Event:
      oneOf:
        - allOf:
            - $ref: "#/components/schemas/UserLoginData"
            - type: object
              required:
                - type
              properties:
                type:
                  type: string
                  enum:
                    - user_login
        - allOf:
            - $ref: "#/components/schemas/UserLogoutData"
            - type: object
              required:
                - type
              properties:
                type:
                  type: string
                  enum:
                    - user_logout
      discriminator:
        propertyName: type
    UserLoginData:
      type: object
      required:
        - user_id
      properties:
        user_id:
          type: integer
          format: int64
    UserLogoutData:
      type: object
      required:
        - reason
      properties:
        reason:
          type: string
    "##);
}
