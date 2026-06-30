//! A unit-variant `enum` derives an independent `type: string, enum: [...]`
//! schema. Variant order matches source declaration order (not
//! alphabetical), matching the on-the-wire string representation that
//! serde produces.
//!
//! The output is identical under both `oas-3-0` and `oas-3-1`.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[test]
fn unit_enum_renders_as_string_enum() {
    let s: frieze::Schemas = frieze::schemas()
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
          - Active
          - Inactive
    ");
}
