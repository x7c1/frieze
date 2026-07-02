//! Runtime defence: `Schemas::build()` rejects a `oneOf` schema whose
//! variant references a non-struct schema (string-enum or another
//! oneOf). The compile-time `IsStructSchema` bound check in the macro
//! is the first line of defence, but the model-side check guards
//! handcrafted `Schema::new_one_of` callers too.

use frieze::SchemasBuilder;
use frieze_model::{Error, OneOfVariant, Schema, SchemaName};

#[test]
fn rejects_oneof_variant_targeting_string_enum() {
    // Build a schema set in which `Event` is a oneOf whose `Mode`
    // variant points at `Status`, but `Status` is registered as a
    // string-enum schema. The macro-side `IsStructSchema` bound check
    // is bypassed here by hand-implementing the trait, so the runtime
    // build check is what enforces "inner must be a struct schema".
    struct DummyStatus;
    impl frieze::Schema for DummyStatus {
        fn name() -> String {
            "Status".to_string()
        }
        fn schema() -> Schema {
            Schema::new_string_enum("Status", vec!["Active".into(), "Inactive".into()]).unwrap()
        }
    }
    impl frieze::IsStructSchema for DummyStatus {}
    impl frieze::Register for DummyStatus {}
    impl frieze::IsRegistrable for DummyStatus {}
    // Intentionally implement `IsStructSchema` on DummyStatus so the
    // macro-side check is bypassed; the runtime check still fires.
    struct DummyEvent;
    impl frieze::Schema for DummyEvent {
        fn name() -> String {
            "Event".to_string()
        }
        fn schema() -> Schema {
            Schema::new_one_of(
                "Event",
                "kind",
                vec![OneOfVariant::new(
                    "Mode",
                    SchemaName::new("Status").unwrap(),
                )],
            )
            .unwrap()
        }
    }
    impl frieze::Register for DummyEvent {}
    impl frieze::IsRegistrable for DummyEvent {}
    let err = SchemasBuilder::new()
        .add::<DummyEvent>()
        .add::<DummyStatus>()
        .build()
        .unwrap_err();
    assert_eq!(
        err,
        Error::OneOfVariantInnerNotStruct {
            schema: "Event".into(),
            variant: "Mode".into(),
            inner: SchemaName::new("Status").unwrap(),
        }
    );
}

#[test]
fn rejects_oneof_variant_targeting_other_oneof() {
    // A oneOf cannot point at another oneOf — the synthesized tag
    // field has nothing to merge into.
    struct DummyInnerStruct;
    impl frieze::Schema for DummyInnerStruct {
        fn name() -> String {
            "InnerStruct".to_string()
        }
        fn schema() -> Schema {
            Schema::new_object(
                "InnerStruct",
                vec![frieze_model::Property::new(
                    "v",
                    frieze_model::PropertyType::Int64,
                    frieze_model::Presence::Required,
                )
                .unwrap()],
            )
            .unwrap()
        }
    }
    impl frieze::IsStructSchema for DummyInnerStruct {}
    impl frieze::Register for DummyInnerStruct {}
    impl frieze::IsRegistrable for DummyInnerStruct {}

    struct DummyInner;
    impl frieze::Schema for DummyInner {
        fn name() -> String {
            "Inner".to_string()
        }
        fn schema() -> Schema {
            Schema::new_one_of(
                "Inner",
                "kind",
                vec![OneOfVariant::new(
                    "Variant",
                    SchemaName::new("InnerStruct").unwrap(),
                )],
            )
            .unwrap()
        }
    }
    impl frieze::IsStructSchema for DummyInner {}
    impl frieze::Register for DummyInner {}
    impl frieze::IsRegistrable for DummyInner {}

    struct DummyOuter;
    impl frieze::Schema for DummyOuter {
        fn name() -> String {
            "Outer".to_string()
        }
        fn schema() -> Schema {
            Schema::new_one_of(
                "Outer",
                "kind",
                vec![OneOfVariant::new("Sub", SchemaName::new("Inner").unwrap())],
            )
            .unwrap()
        }
    }
    impl frieze::Register for DummyOuter {}
    impl frieze::IsRegistrable for DummyOuter {}
    let err = SchemasBuilder::new()
        .add::<DummyOuter>()
        .add::<DummyInner>()
        .add::<DummyInnerStruct>()
        .build()
        .unwrap_err();
    assert_eq!(
        err,
        Error::OneOfVariantInnerNotStruct {
            schema: "Outer".into(),
            variant: "Sub".into(),
            inner: SchemaName::new("Inner").unwrap(),
        }
    );
}
