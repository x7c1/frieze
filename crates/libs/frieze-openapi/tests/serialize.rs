//! Integration tests for the handwritten `Serialize` impls on the
//! `SchemaObject` family.
//!
//! These tests pin the canonical OAS YAML output produced by going
//! directly through `serde_yaml::to_string(&schema)`. They do not run
//! through `frieze-usecase`'s boundary conversion — that path is exercised by the
//! existing snapshots in the `frieze` crate, which must continue to
//! pass byte-for-byte unchanged.
//!
//! Each test pairs a hand-constructed schema value with the expected
//! YAML literal. The expected literals are written so they reflect both
//! the canonical key order
//! (`$ref, type, description, format, minimum, items, required,
//! properties, allOf, oneOf, nullable`) and the OAS 3.0 / 3.1 split for
//! how nullability is encoded.

use indexmap::IndexMap;
use serde_yaml as yaml;

use frieze_openapi::{
    ObjectSchema, OneOfSchema, OneOfVariant, SchemaObject, SchemaType, StringEnumSchema,
};

/// Convenience: render a schema through the handwritten `Serialize`.
fn render<T: serde::Serialize>(value: &T) -> String {
    yaml::to_string(value).expect("YAML serialization must succeed")
}

#[test]
fn object_schema_with_required_and_properties_emits_canonical_key_order() {
    let mut properties: IndexMap<String, ObjectSchema> = IndexMap::new();
    properties.insert(
        "id".to_string(),
        ObjectSchema {
            ty: Some(SchemaType::Integer),
            format: Some("int64".to_string()),
            ..ObjectSchema::empty()
        },
    );
    properties.insert(
        "name".to_string(),
        ObjectSchema {
            ty: Some(SchemaType::String),
            ..ObjectSchema::empty()
        },
    );
    let schema = ObjectSchema {
        ty: Some(SchemaType::Object),
        required: vec!["id".to_string(), "name".to_string()],
        properties: Some(properties),
        ..ObjectSchema::empty()
    };

    let expected = "\
type: object
required:
- id
- name
properties:
  id:
    type: integer
    format: int64
  name:
    type: string
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn object_schema_with_minimum_zero_emits_integer_not_float() {
    // `minimum: 0` (not `0.0`) is the OAS-idiomatic shape for an
    // unsigned scalar bound. The handwritten `Serialize` falls back to
    // a float only when the bound carries fractional information.
    let schema = ObjectSchema {
        ty: Some(SchemaType::Integer),
        format: Some("int32".to_string()),
        minimum: Some(0.0),
        ..ObjectSchema::empty()
    };
    let expected = "\
type: integer
format: int32
minimum: 0
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn object_schema_with_array_items_recurses_through_handwritten_serialize() {
    let schema = ObjectSchema {
        ty: Some(SchemaType::Array),
        items: Some(Box::new(ObjectSchema {
            ty: Some(SchemaType::String),
            ..ObjectSchema::empty()
        })),
        ..ObjectSchema::empty()
    };
    let expected = "\
type: array
items:
  type: string
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-0")]
#[test]
fn object_schema_with_bare_ref_under_oas_3_0_has_no_siblings() {
    // Under OAS 3.0, `$ref` siblings are spec-ignored. The handwritten
    // serializer drops any sibling fields (including `description`) so
    // the wire shape is unambiguous; sibling intent must be expressed
    // via an `allOf` wrap upstream.
    let schema = ObjectSchema {
        reference: Some("#/components/schemas/User".to_string()),
        description: Some("ignored under 3.0".to_string()),
        ..ObjectSchema::empty()
    };
    let expected = "\
$ref: '#/components/schemas/User'
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-1")]
#[test]
fn object_schema_with_ref_and_description_under_oas_3_1_emits_description_sibling() {
    // OAS 3.1 allows sibling `description` next to `$ref`. The
    // handwritten serializer emits it in the canonical post-`$ref`
    // position. Other siblings (e.g. `type`, `format`) are still
    // dropped.
    let schema = ObjectSchema {
        reference: Some("#/components/schemas/User".to_string()),
        description: Some("The current user.".to_string()),
        ty: Some(SchemaType::Object),
        ..ObjectSchema::empty()
    };
    let expected = "\
$ref: '#/components/schemas/User'
description: The current user.
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-0")]
#[test]
fn object_schema_with_nullable_scalar_under_oas_3_0_emits_nullable_true() {
    let schema = ObjectSchema {
        ty: Some(SchemaType::String),
        nullable: Some(true),
        ..ObjectSchema::empty()
    };
    let expected = "\
type: string
nullable: true
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-1")]
#[test]
fn object_schema_with_nullable_scalar_under_oas_3_1_emits_type_sequence() {
    let schema = ObjectSchema {
        ty: Some(SchemaType::String),
        nullable: Some(true),
        ..ObjectSchema::empty()
    };
    let expected = "\
type:
- string
- 'null'
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-0")]
#[test]
fn object_schema_with_allof_wrap_and_nullable_is_the_oas_3_0_nullable_reference_shape() {
    // The OAS 3.0 "nullable reference" shape: `allOf: [{$ref}],
    // nullable: true` on the wrapper.
    let schema = ObjectSchema {
        all_of: Some(vec![ObjectSchema {
            reference: Some("#/components/schemas/Inner".to_string()),
            ..ObjectSchema::empty()
        }]),
        nullable: Some(true),
        ..ObjectSchema::empty()
    };
    let expected = "\
allOf:
- $ref: '#/components/schemas/Inner'
nullable: true
";
    assert_eq!(render(&schema), expected);
}

#[cfg(feature = "oas-3-1")]
#[test]
fn object_schema_with_oneof_wrap_is_the_oas_3_1_nullable_reference_shape() {
    // The OAS 3.1 "nullable reference" shape: `oneOf: [{$ref}, {type:
    // "null"}]`. `nullable` is never emitted under 3.1.
    let schema = ObjectSchema {
        one_of: Some(vec![
            ObjectSchema {
                reference: Some("#/components/schemas/Inner".to_string()),
                ..ObjectSchema::empty()
            },
            ObjectSchema {
                ty: Some(SchemaType::Null),
                ..ObjectSchema::empty()
            },
        ]),
        ..ObjectSchema::empty()
    };
    let expected = "\
oneOf:
- $ref: '#/components/schemas/Inner'
- type: 'null'
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn one_of_schema_emits_synthetic_discriminator_arm_and_omits_mapping() {
    let schema = OneOfSchema::new(
        "kind",
        vec![
            OneOfVariant {
                wire_name: "Login".to_string(),
                inner_reference: "#/components/schemas/LoginData".to_string(),
            },
            OneOfVariant {
                wire_name: "Logout".to_string(),
                inner_reference: "#/components/schemas/LogoutData".to_string(),
            },
        ],
    );
    let expected = "\
oneOf:
- allOf:
  - $ref: '#/components/schemas/LoginData'
  - type: object
    required:
    - kind
    properties:
      kind:
        type: string
        enum:
        - Login
- allOf:
  - $ref: '#/components/schemas/LogoutData'
  - type: object
    required:
    - kind
    properties:
      kind:
        type: string
        enum:
        - Logout
discriminator:
  propertyName: kind
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn one_of_schema_with_description_emits_description_first() {
    let schema = OneOfSchema::new(
        "kind",
        vec![OneOfVariant {
            wire_name: "Only".to_string(),
            inner_reference: "#/components/schemas/Only".to_string(),
        }],
    )
    .with_description(Some("An internally-tagged enum.".to_string()));
    let rendered = render(&schema);
    let first_line = rendered.lines().next().unwrap_or("");
    assert_eq!(first_line, "description: An internally-tagged enum.");
    // The `discriminator` block must carry only `propertyName` — no
    // `mapping` sub-key.
    assert!(
        !rendered.contains("mapping"),
        "discriminator must not emit `mapping`, but the rendered output was:\n{rendered}"
    );
}

#[test]
fn string_enum_schema_emits_type_then_enum_in_canonical_order() {
    let schema = StringEnumSchema::new(vec!["Red".to_string(), "Green".to_string()]);
    let expected = "\
type: string
enum:
- Red
- Green
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn string_enum_schema_with_description_emits_type_description_enum() {
    let schema = StringEnumSchema::new(vec!["Red".to_string(), "Green".to_string()])
        .with_description(Some("A traffic-light hue.".to_string()));
    let expected = "\
type: string
description: A traffic-light hue.
enum:
- Red
- Green
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn schema_object_dispatch_renders_each_variant_through_its_own_serialize() {
    let object = SchemaObject::Object(ObjectSchema {
        ty: Some(SchemaType::String),
        ..ObjectSchema::empty()
    });
    let string_enum = SchemaObject::StringEnum(StringEnumSchema::new(vec!["A".to_string()]));
    let one_of = SchemaObject::OneOf(OneOfSchema::new(
        "kind",
        vec![OneOfVariant {
            wire_name: "X".to_string(),
            inner_reference: "#/components/schemas/X".to_string(),
        }],
    ));

    // Each dispatch arm must produce the same YAML as serializing the
    // inner variant directly.
    let SchemaObject::Object(ref inner_object) = object else {
        unreachable!()
    };
    assert_eq!(render(&object), render(inner_object));

    let SchemaObject::StringEnum(ref inner_string_enum) = string_enum else {
        unreachable!()
    };
    assert_eq!(render(&string_enum), render(inner_string_enum));

    let SchemaObject::OneOf(ref inner_one_of) = one_of else {
        unreachable!()
    };
    assert_eq!(render(&one_of), render(inner_one_of));
}
