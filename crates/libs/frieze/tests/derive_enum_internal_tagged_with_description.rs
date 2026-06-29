//! Doc comments on an internally-tagged enum and its variants compose
//! into the enum-level `description` as a bullet list, mirroring the
//! unit-variant enum behaviour. OAS has no per-variant description slot
//! in `oneOf`, so the bullet list inside the schema's `description` is
//! the only carrier for per-variant prose.
//!
//! Variant names in the bullets use the wire name (post `rename_all` /
//! per-variant `rename`), aligning with the `oneOf` arms.
//!
//! The output is identical under both `oas-3-0` and `oas-3-1`.

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

/// A user session event.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Event {
    /// The user logged in.
    Login(LoginData),
    /// The user logged out.
    Logout(LogoutData),
}

#[test]
fn variant_doc_comments_compose_into_enum_description_bullets() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Event>()
        .add::<LoginData>()
        .add::<LogoutData>()
        .build()
        .expect("schemas build should succeed for valid input");

    // Use `assert_snapshot!` against `to_yaml(...)` so the multi-line
    // description renders as the YAML literal block scalar that frieze
    // emits at the YAML level (the `Value` form would escape `\n` and
    // hide the line structure).
    insta::assert_snapshot!(frieze::to_yaml(&s), @r#"
    Event:
      description: |-
        A user session event.

        - Login: The user logged in.
        - Logout: The user logged out.
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
    "#);
}
