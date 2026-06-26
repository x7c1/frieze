//! `Maybe<T>` under OAS 3.1: the field is dropped from `required` and the
//! inner schema's `type` becomes a 2-element sequence `[<base>, "null"]`.

#![cfg(feature = "oas-3-1")]

use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize, Debug, PartialEq)]
struct Profile {
    id: i64,
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    avatar_url: Maybe<String>,
}

#[test]
fn maybe_field_renders_optional_and_type_null_under_oas_3_1() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Profile:
      type: object
      properties:
        id:
          type: integer
          format: int64
        avatar_url:
          type:
            - string
            - "null"
      required:
        - id
    "###);
}
