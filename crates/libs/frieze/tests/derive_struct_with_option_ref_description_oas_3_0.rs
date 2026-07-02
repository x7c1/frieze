//! A nullable nested reference (`Option<U>`, serde default) with a
//! field-level doc. Under OAS 3.0 the `allOf` wrap is already required
//! to carry `nullable: true`; the `description` sits on the same outer
//! wrapper next to `allOf` and `nullable`.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Image {
    url: String,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Profile {
    /// Optional avatar associated with this profile.
    avatar: Option<Image>,
}

#[test]
fn option_ref_with_description_carries_description_on_all_of_wrap_under_oas_3_0() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .add::<Image>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Image:
          type: object
          required:
          - url
          properties:
            url:
              type: string
        Profile:
          type: object
          required:
          - avatar
          properties:
            avatar:
              description: Optional avatar associated with this profile.
              allOf:
              - $ref: '#/components/schemas/Image'
              nullable: true
    ");
}
