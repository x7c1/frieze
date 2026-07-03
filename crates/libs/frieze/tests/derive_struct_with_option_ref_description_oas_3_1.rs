//! Same source as `derive_struct_with_option_ref_description_oas_3_0` —
//! under OAS 3.1 the nullable reference is encoded as
//! `oneOf: [$ref, {type: null}]` and the description sits on the
//! `oneOf` wrapper, not inside the array.

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
fn option_ref_with_description_carries_description_on_one_of_wrap_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Profile>()
        .add::<Image>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_1(s), @"
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
              oneOf:
              - $ref: '#/components/schemas/Image'
              - type: 'null'
    ");
}
