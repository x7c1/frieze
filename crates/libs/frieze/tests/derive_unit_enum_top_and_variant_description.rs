//! Enum-level + per-variant docs compose into "top doc, blank line,
//! bullet list of variants" per the design table. Each bullet uses the
//! OAS output name (matching the `enum` array element 1:1).

use frieze::Schema;

/// Lifecycle state of an entity.
#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    /// The entity is currently active.
    Active,
    /// The entity is no longer active.
    Inactive,
}

#[test]
fn enum_top_and_all_variant_docs_compose_into_description() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Status>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    Status:
      type: string
      description: "Lifecycle state of an entity.\n\n- Active: The entity is currently active.\n- Inactive: The entity is no longer active."
      enum:
        - Active
        - Inactive
    "#);
}
