//! A nullable nested reference (`Option<U>`, serde default) with a
//! field-level doc. Under OAS 3.0 the `allOf` wrap is already required
//! to carry `nullable: true`; the `description` sits on the same outer
//! wrapper next to `allOf` and `nullable`.

#![cfg(feature = "oas-3-0")]

use frieze::Schema;

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
    let s: frieze::Schemas = frieze::schemas()
        .add::<Profile>()
        .add::<Image>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
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
            - $ref: "#/components/schemas/Image"
          nullable: true
    "###);
}
