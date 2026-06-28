//! Field-level `///` doc comment rides on the renamed wire name —
//! the property's `description` lands under the renamed key, not under
//! the Rust identifier.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    /// The user's external identifier.
    #[serde(rename = "userId")]
    user_id: i64,
}

#[test]
fn field_description_attaches_under_renamed_key() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(frieze::to_yaml(&s), @r#"
    User:
      type: object
      required:
      - userId
      properties:
        userId:
          type: integer
          description: The user's external identifier.
          format: int64
    "#);
}
