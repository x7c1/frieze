//! `Maybe<User>` maps to branch ④ (optional + nullable). The field is
//! dropped from `required` and the reference is rendered through the
//! OAS-3.1 nullable-ref wrap (`oneOf: [$ref, {type: "null"}]`).

#![cfg(feature = "oas-3-1")]

use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Profile {
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    user: Maybe<User>,
}

#[test]
fn maybe_nested_renders_as_optional_nullable_ref_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Profile:
      type: object
      properties:
        user:
          oneOf:
            - $ref: "#/components/schemas/User"
            - type: "null"
      required: []
    User:
      type: object
      properties:
        id:
          type: integer
          format: int64
      required:
        - id
    "###);
}
