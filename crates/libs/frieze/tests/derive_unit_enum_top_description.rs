//! Enum-level `///` doc with no per-variant docs renders only the
//! enum's own description — no bullet list is appended.

use frieze::Schema;

/// Lifecycle state of an entity.
#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[test]
fn enum_top_doc_only_renders_without_variant_list() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Status:
      type: string
      description: Lifecycle state of an entity.
      enum:
        - Active
        - Inactive
    "###);
}
