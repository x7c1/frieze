//! With no enum-level doc, per-variant docs compose into a bullet list
//! that becomes the enum schema's `description`. Variants without doc
//! still appear in the `enum` array but are omitted from the bullet
//! list (a bare `- name:` row would be noise for readers).

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
enum Color {
    /// The crimson hue.
    Red,
    Green,
    /// The deep blue.
    Blue,
}

#[test]
fn variant_docs_only_render_as_bullet_list_description() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Color>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Color:
          type: string
          description: |-
            - Red: The crimson hue.
            - Blue: The deep blue.
          enum:
          - Red
          - Green
          - Blue
    ");
}
