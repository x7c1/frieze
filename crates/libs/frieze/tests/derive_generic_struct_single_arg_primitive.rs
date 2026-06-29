//! Schema name composition for a single-argument generic struct
//! instantiated with a primitive: `Container<i64>` produces
//! `Int64_Container`, following the suffix-form rule
//! `<Arg>_<Base>`.
//!
//! The full `Schemas::add::<Container<i64>>().build()` flow is not
//! exercised here because the emitted property reference is
//! `$ref: Int64`, and primitives cannot be registered on a
//! `SchemasBuilder` (the `IsRegistrable` marker rejects
//! `Schemas::add::<i64>()`). Resolving primitive-scalar references to
//! inline `PropertyType`s would let `Container<i64>` round-trip through
//! `Schemas` end-to-end; that is left as a follow-up.
//!
//! `Container<User>` (user-type instantiation) is exercised end-to-end
//! by `derive_generic_struct_single_arg_user.rs`.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Container<T> {
    value: T,
}

#[test]
fn container_i64_name_uses_suffix_form() {
    assert_eq!(<Container<i64> as Schema>::name(), "Int64_Container");
}

#[test]
fn container_string_name_uses_suffix_form() {
    assert_eq!(<Container<String> as Schema>::name(), "String_Container");
}

#[test]
fn container_bool_name_uses_suffix_form() {
    assert_eq!(<Container<bool> as Schema>::name(), "Boolean_Container");
}
