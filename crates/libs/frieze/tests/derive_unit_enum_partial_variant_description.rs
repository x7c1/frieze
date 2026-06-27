//! Enum-level doc + only some variants with their own doc:
//!
//! - bullet list appears, with rows for documented variants only.
//! - the `enum` array still contains every variant in declaration order.

use frieze::Schema;

/// Lifecycle state of an entity.
#[derive(Schema)]
#[allow(dead_code)]
enum Status {
    /// The entity is currently active.
    Active,
    Pending,
    /// The entity is no longer active.
    Inactive,
}

#[test]
fn missing_variant_docs_are_omitted_from_the_bullet_list() {
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
        - Pending
        - Inactive
    "#);
}
