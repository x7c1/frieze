//! Container-level `#[serde(rename_all = "kebab-case")]` rewrites the
//! variant names to lower-case hyphen-separated strings. The conversion
//! mirrors serde's variant-rule behaviour: an upper-case letter (after
//! the first character) introduces the separator.

use frieze::Schema;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
enum Status {
    Active,
    InactiveSince,
}

#[test]
fn rename_all_kebab_case_hyphenates_variants() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Status:
          type: string
          enum:
          - active
          - inactive-since
    ");
}
