//! End-to-end (negative): with the `inventory` feature disabled the
//! `#[frieze(namespace)]` attribute is a transparent pass-through and
//! the OAS keys are bare names.
//!
//! Design F1 / H5: the macro itself is always built (it is part of
//! the proc-macro crate which has no feature gate), but the
//! `inventory_namespace!` helper compiles to nothing without the
//! `inventory` feature. Consumers who write a namespace declaration
//! and later opt out of `inventory` see the namespace lookup
//! short-circuit; their code keeps compiling and OAS keys stay
//! bare.
//!
//! Gated to the no-inventory build path with
//! `#[cfg(not(feature = "inventory"))]` so the test compiles only
//! under that configuration. The matrix runs include
//! `--features oas-3-0` and `--features oas-3-1` (without
//! `inventory`); under both this test is active.

#![cfg(not(feature = "inventory"))]

use frieze::Schema;

#[frieze::frieze(namespace)]
pub mod v1 {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct User {
        pub id: i64,
    }
}

#[test]
fn namespace_attr_is_passthrough_when_inventory_disabled() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<v1::User>()
        .build()
        .expect("schemas builder closes over `User` alone");

    // With `inventory` off, `compose_schema_name` is the identity:
    // the OAS key is the bare `User`, identical to the pre-PR-1.5
    // emission. The attribute itself parsed and the surrounding code
    // compiled — the side channel just had nowhere to land.
    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    User:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}

// Reference `Schema` to silence the unused-import lint when this is
// the only mention.
const _: fn() = || {
    let _ = <v1::User as Schema>::name();
};
