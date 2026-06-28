//! `#[serde(rename = "literal")]` on a single variant overrides the
//! container-level `rename_all` for that one entry. The remaining
//! variants still follow the container rule.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum Status {
    Active,
    // Container rule would produce `inactive_since`; the individual
    // rename takes precedence.
    #[serde(rename = "gone")]
    InactiveSince,
}

#[test]
fn individual_variant_rename_takes_precedence_over_rename_all() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      enum:
        - active
        - gone
    "###);
}
