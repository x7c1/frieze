//! End-to-end: a generic `Container<T>` inside a `#[frieze(namespace)]`
//! mod composes its OAS key as `v1.Int64_Container` when instantiated
//! with `T = i64`.
//!
//! Exercises the order of operations: generic-suffix composition
//! (`<Arg>_<Base>`) happens first, then the namespace prefix is `.` -joined onto the
//! composed base. Primitive arguments (`i64` → `"Int64"`) do not
//! carry namespaces of their own (they do not implement `Schema`
//! through a derive site, so `module_path!()` does not enter the
//! picture for them), keeping the composed name compact.

#![cfg(feature = "inventory")]

#[frieze::frieze(namespace)]
pub mod v1 {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct Container<T> {
        pub item: T,
    }

    // Anchor `Container<i64>` on a non-generic root so
    // `from_inventory()` walks into it transitively (a generic type
    // alone cannot live in an inventory submission, by design).
    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct Wrapper {
        pub inner: Container<i64>,
    }
}

#[test]
fn namespace_prefix_composes_with_generic_suffix() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    // Inventory submissions:
    //   {parent: "derive_namespace_with_generic", local: "v1"}
    //   `Wrapper` (non-generic root from the derive)
    //
    // Reaching from `Wrapper` into `Container<i64>` produces the
    // generic-suffix base `"Int64_Container"`, then
    // `compose_schema_name(module_path!(), "Int64_Container")`
    // prefixes `v1`, yielding `v1.Int64_Container`.
    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    v1.Int64_Container:
      type: object
      required:
        - item
      properties:
        item:
          type: integer
          format: int64
    v1.Wrapper:
      type: object
      required:
        - inner
      properties:
        inner:
          $ref: "#/components/schemas/v1.Int64_Container"
    "##);
}
