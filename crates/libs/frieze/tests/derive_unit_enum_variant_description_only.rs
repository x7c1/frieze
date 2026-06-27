//! With no enum-level doc, per-variant docs compose into a bullet list
//! that becomes the enum schema's `description`. Variants without doc
//! still appear in the `enum` array but are omitted from the bullet
//! list (a bare `- name:` row would be noise for readers).

use frieze::Schema;

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
    let s: frieze::Schemas = frieze::schemas()
        .add::<Color>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    Color:
      type: string
      description: "- Red: The crimson hue.\n- Blue: The deep blue."
      enum:
        - Red
        - Green
        - Blue
    "#);
}
