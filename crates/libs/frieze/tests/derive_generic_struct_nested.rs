//! Nested generic-arg composition: `Container<Container<i64>>` produces
//! `Int64_Container_Container` — each layer's `name()` recursively
//! composes the layer-inner's name with its own base, yielding a flat
//! concatenation without doubled separators.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[test]
fn container_of_container_of_i64_name_collapses_recursively() {
    assert_eq!(
        <Container<Container<i64>> as Schema>::name(),
        "Int64_Container_Container"
    );
}

#[test]
fn container_of_container_of_string_name_collapses_recursively() {
    assert_eq!(
        <Container<Container<String>> as Schema>::name(),
        "String_Container_Container"
    );
}
