//! [`Schema`] / [`Register`] implementations for primitive scalar types
//! (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `String`).
//!
//! Primitive scalars implement [`Schema`] and [`Register`] so they can
//! appear as generic arguments — `Box<i64>`, `Page<String>`, etc. — and
//! let generic derive output keep uniform trait bounds across struct,
//! enum, and primitive arguments.
//!
//! Primitives intentionally **do not** implement [`crate::IsRegistrable`].
//! This is the compile-time guard that rejects
//! `Schemas::add::<i64>()` — primitives are not standalone OAS schema
//! entries. The `Schema::Scalar` arm in the boundary conversion is the
//! defensive secondary guard.
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
//!
//! Optional features add further leaf scalars to that table — not Rust
//! primitives, but types that behave like one here:
//!
//! | Rust                            | name       | feature    |
//! |---------------------------------|------------|------------|
//! | `uuid::Uuid`                    | `Uuid`     | `uuid1`    |
//! | `chrono::DateTime<Utc>`         | `DateTime` | `chrono04` |
//! | `chrono::DateTime<FixedOffset>` | `DateTime` | `chrono04` |
//! | `chrono::DateTime<Local>`       | `DateTime` | `chrono04` |
//! | `chrono::NaiveDate`             | `Date`     | `chrono04` |
//!
//! Only the impls are feature-gated — the matching leaf variants and
//! their reserved names live unconditionally in `frieze-model`.
//!
//! `chrono::DateTime<Tz>` gets those three impls and no more, even
//! though chrono can *serialize* every `Tz`. RFC 3339 carries a numeric
//! offset and no zone name, so `Utc`, `FixedOffset` and `Local` are
//! exactly the time zones chrono implements `Deserialize` for; pinning
//! the impl set to them makes "has a frieze schema" coincide with
//! "round-trips the wire". A field typed with any other `Tz`
//! (`chrono_tz::Tz`, say) fails at derive time with the ordinary
//! missing-`Schema` trait-bound error, exactly like every other
//! unsupported type, instead of obtaining a schema that the struct's own
//! `#[derive(Deserialize)]` cannot honour.
//!
//! `chrono::NaiveDateTime` is deliberately absent: it carries no UTC
//! offset, so serde writes it as a naive `2015-09-18T23:56:04` rather
//! than an RFC 3339 `date-time`, and declaring `format: date-time` for
//! it would break the rule that the declared format matches the wire
//! shape.

use frieze_model::{PropertyType, Schema as ModelSchema};

use crate::register::Register;
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
        }
        impl Register for $ty {
            fn register_into(_builder: &mut SchemasBuilder) {
                // Primitive scalars are inlined at the boundary
                // conversion and never registered as standalone
                // entries under `#/components/schemas`. The override
                // stays a no-op so transitive `register_into` calls
                // from derived schemas can be emitted uniformly
                // (`<#ty as Register>::register_into`) without the
                // macro special-casing primitive field types.
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

#[cfg(feature = "uuid1")]
impl_primitive_schema!(uuid::Uuid, "Uuid", Uuid);

#[cfg(feature = "chrono04")]
impl_primitive_schema!(chrono::NaiveDate, "Date", Date);

// One impl per time zone rather than a single impl generic over `Tz`;
// the module doc explains why the set is pinned to the three time zones
// chrono can deserialize.
//
// All three share the schema name `"DateTime"` instead of composing the
// type argument the way generic user structs do (`Container<i64>` →
// `Int64_Container`). That is what makes them usable as fields: the
// derive emits `Reference(<DateTime<Utc> as Schema>::name())`, and only
// the bare name `DateTime` is inlined as the leaf scalar by the
// boundary. A per-`Tz` name would be wrong on the wire as well, since
// all three describe the same `{type: string, format: date-time}`
// shape.
#[cfg(feature = "chrono04")]
impl_primitive_schema!(chrono::DateTime<chrono::Utc>, "DateTime", DateTime);
#[cfg(feature = "chrono04")]
impl_primitive_schema!(chrono::DateTime<chrono::FixedOffset>, "DateTime", DateTime);
#[cfg(feature = "chrono04")]
impl_primitive_schema!(chrono::DateTime<chrono::Local>, "DateTime", DateTime);

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

    #[cfg(feature = "uuid1")]
    #[test]
    fn uuid_name_is_uuid() {
        assert_eq!(<uuid::Uuid as Schema>::name(), "Uuid");
    }

    #[cfg(feature = "uuid1")]
    #[test]
    fn uuid_schema_is_scalar_uuid() {
        let schema = <uuid::Uuid as Schema>::schema();
        let expected = ModelSchema::Scalar(ScalarSchema::new(PropertyType::Uuid).unwrap());
        assert_eq!(schema, expected);
    }

    #[cfg(feature = "chrono04")]
    #[test]
    fn date_time_name_is_date_time_for_every_supported_time_zone() {
        assert_eq!(
            <chrono::DateTime<chrono::Utc> as Schema>::name(),
            "DateTime"
        );
        assert_eq!(
            <chrono::DateTime<chrono::FixedOffset> as Schema>::name(),
            "DateTime"
        );
        assert_eq!(
            <chrono::DateTime<chrono::Local> as Schema>::name(),
            "DateTime"
        );
    }

    #[cfg(feature = "chrono04")]
    #[test]
    fn date_time_schema_is_scalar_date_time_for_every_supported_time_zone() {
        let expected = ModelSchema::Scalar(ScalarSchema::new(PropertyType::DateTime).unwrap());
        assert_eq!(
            <chrono::DateTime<chrono::Utc> as Schema>::schema(),
            expected
        );
        assert_eq!(
            <chrono::DateTime<chrono::FixedOffset> as Schema>::schema(),
            expected
        );
        assert_eq!(
            <chrono::DateTime<chrono::Local> as Schema>::schema(),
            expected
        );
    }

    #[cfg(feature = "chrono04")]
    #[test]
    fn naive_date_name_is_date() {
        assert_eq!(<chrono::NaiveDate as Schema>::name(), "Date");
    }

    #[cfg(feature = "chrono04")]
    #[test]
    fn naive_date_schema_is_scalar_date() {
        let schema = <chrono::NaiveDate as Schema>::schema();
        let expected = ModelSchema::Scalar(ScalarSchema::new(PropertyType::Date).unwrap());
        assert_eq!(schema, expected);
    }
}
