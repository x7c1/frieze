//! End-to-end: `#[frieze(namespace)] pub mod v1 { mod foo; pub use foo::Foo; }`
//! collapses `foo` (no namespace) but retains `v1`, producing
//! `v1.Foo` — not `v1.foo.Foo`.
//!
//! Exercises the prefix-walk fold: it drops segments whose
//! full path is *not* declared as a namespace, even when they sit
//! between two ancestors that *are* declared. Here only `v1` is
//! declared; `v1::foo` is implementation detail and is folded away.

#![cfg(feature = "inventory")]

#[frieze::frieze(namespace)]
pub mod v1 {
    // Implementation-detail submodule, no namespace attribute. Its
    // segment must be dropped during composition so the final OAS
    // key reads `v1.Foo`, not `v1.foo.Foo`.
    mod foo {
        use frieze::Schema;

        #[derive(Schema)]
        #[allow(dead_code)]
        pub struct Foo {
            pub id: i64,
        }
    }

    pub use foo::Foo;
}

// Force `Foo` to be reachable from the test root so the unused-import
// lint stays quiet under `--features inventory` (where the test does
// not otherwise reference the re-export beyond the inventory walk).
#[allow(dead_code)]
type _ForceReachable = v1::Foo;

#[test]
fn intermediate_non_namespace_mod_is_collapsed() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    // Inventory submissions: just `{parent: "derive_namespace_with_facade_inside", local: "v1"}`.
    // Module path at `Foo::name()` derive site:
    //   "derive_namespace_with_facade_inside::v1::foo"
    // Prefix walk: only `v1` is declared, so `foo` is dropped. Final
    // composed name: `v1.Foo`.
    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    v1.Foo:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}
