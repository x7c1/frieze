//! End-to-end: two layers of `#[frieze(namespace)]` (`v1` containing
//! `v2`) fold into the dotted OAS key `v1.v2.User`.
//!
//! Exercises the central claim of the prefix walk: every segment in
//! `compose_schema_name` whose full path is a declared namespace is
//! retained, and the retained segments are joined by `.`. Two-level
//! nesting is the smallest interesting test of "more than one
//! namespace contributes".
//!
//! Per-binary inventory isolation: this test binary defines two
//! namespaces (`v1`, `v1::v2`) and exactly one `#[derive(Schema)]`
//! type (`User`).

#![cfg(feature = "inventory")]

#[frieze::frieze(namespace)]
pub mod v1 {
    #[frieze::frieze(namespace)]
    pub mod v2 {
        use frieze::Schema;

        #[derive(Schema)]
        #[allow(dead_code)]
        pub struct User {
            pub id: i64,
        }
    }
}

#[test]
fn nested_namespaces_chain_with_dots() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    // Inventory submissions:
    //   {parent: "derive_namespace_nested",       local: "v1"}
    //   {parent: "derive_namespace_nested::v1",   local: "v2"}
    // Module path at `User::name()` derive site:
    //   "derive_namespace_nested::v1::v2"
    // Prefix walk keeps `v1` then `v2`, dropping the crate name; the
    // base "User" is `.` -joined onto the chain.
    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    v1.v2.User:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}
