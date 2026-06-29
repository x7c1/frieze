//! [`Schema`] implementations for primitive scalar types (`i32`, `i64`,
//! `u32`, `u64`, `f32`, `f64`, `bool`, `String`).
//!
//! Primitive scalars implement [`Schema`] so they can appear as generic
//! arguments — `Box<i64>`, `Page<String>`, etc. — and let generic
//! derive output keep a uniform `T: Schema` trait bound across struct,
//! enum, and primitive arguments.
//!
//! Primitives intentionally **do not** implement [`crate::IsRegistrable`].
//! This is the compile-time guard that rejects
//! `Schemas::add::<i64>()` — primitives are not standalone OAS schema
//! entries. The [`crate::Schema::Scalar`] arm in
//! [`crate::to_value::to_openapi`] is the defensive secondary guard.
//!
//! Schema names follow OAS type/format conventions:
//!
//! | Rust   | name      |
//! |--------|-----------|
//! | `i32`  | `Int32`   |
//! | `i64`  | `Int64`   |
//! | `u32`  | `UInt32`  |
//! | `u64`  | `UInt64`  |
//! | `f32`  | `Float`   |
//! | `f64`  | `Double`  |
//! | `bool` | `Boolean` |
//! | `String` | `String` |

use frieze_model::{PropertyType, Schema as ModelSchema};

use crate::schema::Schema;
use crate::schemas_builder::SchemasBuilder;

const PRIMITIVE_SCALAR_INVARIANT_MSG: &str =
    "frieze: primitive scalar satisfies the leaf-PropertyType invariant by construction";

macro_rules! impl_primitive_schema {
    ($ty:ty, $name:literal, $variant:ident) => {
        impl Schema for $ty {
            fn name() -> ::std::string::String {
                ::std::string::String::from($name)
            }
            fn schema() -> ModelSchema {
                ModelSchema::new_scalar(PropertyType::$variant)
                    .expect(PRIMITIVE_SCALAR_INVARIANT_MSG)
            }
            fn register_into(_builder: &mut SchemasBuilder) {
                // Primitive scalars are inlined at the boundary
                // conversion in `to_value` and never registered as
                // standalone entries under `#/components/schemas`. The
                // override stays a no-op so transitive
                // `register_into` calls from derived schemas can be
                // emitted uniformly (`<#ty as Schema>::register_into`)
                // without the macro special-casing primitive field
                // types.
            }
        }
    };
}

impl_primitive_schema!(i32, "Int32", Int32);
impl_primitive_schema!(i64, "Int64", Int64);
impl_primitive_schema!(u32, "UInt32", UInt32);
impl_primitive_schema!(u64, "UInt64", UInt64);
impl_primitive_schema!(f32, "Float", Float);
impl_primitive_schema!(f64, "Double", Double);
impl_primitive_schema!(bool, "Boolean", Boolean);
impl_primitive_schema!(String, "String", String);

#[cfg(test)]
mod tests {
    use super::*;
    use frieze_model::ScalarSchema;

    #[test]
    fn i32_name_is_int32() {
        assert_eq!(<i32 as Schema>::name(), "Int32");
    }

    #[test]
    fn i64_name_is_int64() {
        assert_eq!(<i64 as Schema>::name(), "Int64");
    }

    #[test]
    fn u32_name_is_uint32() {
        assert_eq!(<u32 as Schema>::name(), "UInt32");
    }

    #[test]
    fn u64_name_is_uint64() {
        assert_eq!(<u64 as Schema>::name(), "UInt64");
    }

    #[test]
    fn f32_name_is_float() {
        assert_eq!(<f32 as Schema>::name(), "Float");
    }

    #[test]
    fn f64_name_is_double() {
        assert_eq!(<f64 as Schema>::name(), "Double");
    }

    #[test]
    fn bool_name_is_boolean() {
        assert_eq!(<bool as Schema>::name(), "Boolean");
    }

    #[test]
    fn string_name_is_string() {
        assert_eq!(<String as Schema>::name(), "String");
    }

    #[test]
    fn i64_schema_is_scalar_int64() {
        let schema = <i64 as Schema>::schema();
        let expected = ModelSchema::Scalar(ScalarSchema::new(PropertyType::Int64).unwrap());
        assert_eq!(schema, expected);
    }

    #[test]
    fn string_schema_is_scalar_string() {
        let schema = <String as Schema>::schema();
        let expected = ModelSchema::Scalar(ScalarSchema::new(PropertyType::String).unwrap());
        assert_eq!(schema, expected);
    }
}
