//! Primitive scalar types (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`,
//! `bool`, `String`) implement [`frieze::Schema`] so they can appear as
//! generic arguments. Their `name()` follows the OAS type/format
//! convention (`Int64`, `UInt32`, `Boolean`, ...).
//!
//! Primitives do NOT implement `IsRegistrable`, so
//! `Schemas::add::<i64>()` is a compile error — that aspect is asserted
//! by the trybuild fixture `tests/ui/add_primitive_rejected.rs`.

use frieze::Schema;

#[test]
fn primitive_names_match_oas_format_convention() {
    assert_eq!(<i32 as Schema>::name(), "Int32");
    assert_eq!(<i64 as Schema>::name(), "Int64");
    assert_eq!(<u32 as Schema>::name(), "UInt32");
    assert_eq!(<u64 as Schema>::name(), "UInt64");
    assert_eq!(<f32 as Schema>::name(), "Float");
    assert_eq!(<f64 as Schema>::name(), "Double");
    assert_eq!(<bool as Schema>::name(), "Boolean");
    assert_eq!(<String as Schema>::name(), "String");
}

#[test]
fn primitive_schema_is_scalar_variant() {
    // The Scalar variant is the only path by which primitives are
    // expressible in `frieze_model::Schema`; the leaf PropertyType
    // matches the Rust scalar.
    use frieze::PropertyType;
    use frieze_model::{ScalarSchema, Schema as ModelSchema};

    let cases: &[(ModelSchema, PropertyType)] = &[
        (<i32 as Schema>::schema(), PropertyType::Int32),
        (<i64 as Schema>::schema(), PropertyType::Int64),
        (<u32 as Schema>::schema(), PropertyType::UInt32),
        (<u64 as Schema>::schema(), PropertyType::UInt64),
        (<f32 as Schema>::schema(), PropertyType::Float),
        (<f64 as Schema>::schema(), PropertyType::Double),
        (<bool as Schema>::schema(), PropertyType::Boolean),
        (<String as Schema>::schema(), PropertyType::String),
    ];
    for (schema, expected_ty) in cases {
        let expected = ModelSchema::Scalar(ScalarSchema::new(expected_ty.clone()).unwrap());
        assert_eq!(schema, &expected, "mismatch for primitive scalar");
    }
}
