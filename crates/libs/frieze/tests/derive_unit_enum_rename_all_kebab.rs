//! Container-level `#[serde(rename_all = "kebab-case")]` rewrites the
//! variant names to lower-case hyphen-separated strings. The conversion
//! mirrors serde's variant-rule behaviour: an upper-case letter (after
//! the first character) introduces the separator.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
enum Status {
    Active,
    InactiveSince,
}

#[test]
fn rename_all_kebab_case_hyphenates_variants() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      enum:
        - active
        - inactive-since
    "###);
}
