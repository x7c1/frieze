//! A variant-level `#[serde(rename = "literal")]` rewrites the value
//! emitted into the `enum` array for that variant. Other variants are
//! emitted from the Rust identifier unchanged.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
enum Status {
    Active,
    #[serde(rename = "off")]
    Inactive,
}

#[test]
fn variant_rename_rewrites_the_enum_value() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      enum:
        - Active
        - "off"
    "###);
}
