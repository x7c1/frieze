//! `Maybe<T>` produces the optional + nullable shape (`Maybe<T>` in
//! `docs/field-shapes.md`): the field is dropped from the schema's
//! `required` array **and** the inner schema is marked nullable.
//!
//! The struct also exercises serde round-tripping: a `Maybe<String>` field
//! paired with `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`
//! deserializes a missing key as `Maybe::Missing` and serializes it back
//! to a missing key. This verifies the documented user-facing attribute
//! pairing.

use frieze::Schema;
use frieze_model::Maybe;
use serde::{Deserialize, Serialize};

mod common;

#[derive(Schema, Serialize, Deserialize, Debug, PartialEq)]
struct Profile {
    id: i64,
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    avatar_url: Maybe<String>,
}

#[test]
fn maybe_field_renders_optional_and_nullable_under_oas_3_0() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_0(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Profile:
          type: object
          required:
          - id
          properties:
            id:
              type: integer
              format: int64
            avatar_url:
              type: string
              nullable: true
    ");
}

#[test]
fn maybe_field_distinguishes_missing_null_and_present_through_serde() {
    let missing: Profile = serde_json::from_str(r#"{"id": 1}"#).unwrap();
    let null: Profile = serde_json::from_str(r#"{"id": 1, "avatar_url": null}"#).unwrap();
    let present: Profile = serde_json::from_str(r#"{"id": 1, "avatar_url": "u"}"#).unwrap();

    assert_eq!(missing.avatar_url, Maybe::Missing);
    assert_eq!(null.avatar_url, Maybe::Null);
    assert_eq!(present.avatar_url, Maybe::Present("u".into()));

    // Missing should be omitted on the wire (key absent), Null stays as
    // `null`, Present serializes the inner value.
    assert_eq!(serde_json::to_string(&missing).unwrap(), r#"{"id":1}"#);
    assert_eq!(
        serde_json::to_string(&null).unwrap(),
        r#"{"id":1,"avatar_url":null}"#
    );
    assert_eq!(
        serde_json::to_string(&present).unwrap(),
        r#"{"id":1,"avatar_url":"u"}"#
    );
}
