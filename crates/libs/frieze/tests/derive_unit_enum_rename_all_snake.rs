//! Container-level `#[serde(rename_all = "snake_case")]` rewrites the
//! variant names emitted into the `enum` array so that the OAS schema
//! matches the form serde will produce on the wire.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Status {
    Active,
    InactiveSince,
}

#[test]
fn rename_all_snake_case_lowercases_and_underscores_variants() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      enum:
        - active
        - inactive_since
    "###);
}
