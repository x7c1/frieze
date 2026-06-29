//! `Schemas::add::<Container<i64>>().build()` succeeds without any
//! `Int64` registration.
//!
//! Generic derive cannot tell at expansion time whether `T` is a
//! primitive, so monomorphisation of `Container<T>` for `T = i64` emits
//! `PropertyType::Reference(SchemaName("Int64"))` for the inner `value`
//! field. Primitives implement `Schema` but intentionally not
//! `IsRegistrable`, so `Schemas::add::<i64>()` is rejected at compile
//! time and the `Int64` name is never present in `schemas.by_name`.
//! The build-time reference walk treats primitive names as resolved by
//! looking them up against the primitive-name table; the boundary
//! conversion inlines them at the leaf.
//!
//! Without this resolution path, `build()` would return
//! `Err(UnresolvedReference("Int64"))`. This test would have failed in
//! that earlier state and locks in the build-time success.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[test]
fn build_succeeds_for_container_over_i64() {
    let _ = frieze::schemas()
        .add::<Container<i64>>()
        .build()
        .expect("schemas build should resolve the inner `Int64` reference inline");
}

#[test]
fn build_succeeds_for_container_over_string() {
    let _ = frieze::schemas()
        .add::<Container<String>>()
        .build()
        .expect("schemas build should resolve the inner `String` reference inline");
}

#[test]
fn build_succeeds_for_container_over_bool() {
    let _ = frieze::schemas()
        .add::<Container<bool>>()
        .build()
        .expect("schemas build should resolve the inner `Boolean` reference inline");
}
