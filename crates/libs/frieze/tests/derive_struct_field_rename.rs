//! `#[serde(rename = "literal")]` on a struct field rewrites both the
//! `properties` key and the `required` array entry to the renamed
//! form. The Rust identifier (`user_id`) is invisible on the wire.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    #[serde(rename = "userId")]
    user_id: i64,
    email: String,
}

#[test]
fn field_rename_rewrites_properties_and_required() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    User:
      type: object
      required:
        - userId
        - email
      properties:
        userId:
          type: integer
          format: int64
        email:
          type: string
    "###);
}
