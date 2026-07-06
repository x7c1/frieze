//! Integration tests for the canonical (version-neutral) `Serialize`
//! path on the `SchemaObject` family and `Components`.
//!
//! The canonical form is the derived `Serialize`: keys mirror the
//! struct fields one-to-one (`$ref`, `description` and `nullable` as
//! plain siblings, string enums as `{values}`, internally-tagged
//! `oneOf` schemas as `{tag, variants}`), and the output round-trips
//! through the derived `Deserialize`. It is the format for
//! machine-readable dumps of `Components` exchanged between tools.
//!
//! The OAS wire form — where the OAS 3.0 / 3.1 encoding split is
//! applied — is a separate path: it is produced only when a value
//! rides inside a serialized `Document`, and is pinned by the unit
//! tests of the versioned emitter in `frieze-openapi` plus the e2e
//! snapshots in the `frieze` crate.

use indexmap::IndexMap;
use serde_yaml as yaml;
use std::collections::BTreeMap;

use frieze_openapi::{
    Components, ObjectSchema, OneOfSchema, OneOfVariant, SchemaObject, SchemaType, StringEnumSchema,
};

/// Convenience: render a value through the canonical (derived)
/// `Serialize`.
fn render<T: serde::Serialize>(value: &T) -> String {
    yaml::to_string(value).expect("YAML serialization must succeed")
}

#[test]
fn canonical_object_schema_mirrors_struct_fields_in_declaration_order() {
    let mut properties: IndexMap<String, ObjectSchema> = IndexMap::new();
    properties.insert(
        "id".to_string(),
        ObjectSchema {
            ty: Some(SchemaType::Integer),
            format: Some("int64".to_string()),
            ..ObjectSchema::empty()
        },
    );
    let schema = ObjectSchema {
        ty: Some(SchemaType::Object),
        required: vec!["id".to_string()],
        properties: Some(properties),
        ..ObjectSchema::empty()
    };

    let expected = "\
type: object
required:
- id
properties:
  id:
    type: integer
    format: int64
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn canonical_reference_keeps_description_and_nullable_as_plain_siblings() {
    // The canonical form does not apply any OAS version's
    // `$ref`-sibling rules: the three fields serialize exactly as
    // stored, which is what makes the dump version-neutral and
    // losslessly round-trippable.
    let schema = ObjectSchema {
        reference: Some("#/components/schemas/User".to_string()),
        description: Some("The current user.".to_string()),
        nullable: Some(true),
        ..ObjectSchema::empty()
    };

    let expected = "\
$ref: '#/components/schemas/User'
description: The current user.
nullable: true
";
    assert_eq!(render(&schema), expected);

    let parsed: ObjectSchema = yaml::from_str(expected).expect("canonical form must parse back");
    assert_eq!(parsed, schema);
}

#[test]
fn canonical_string_enum_serializes_as_values_and_description() {
    let schema = StringEnumSchema::new(vec!["Red".to_string(), "Green".to_string()])
        .with_description(Some("A traffic-light hue.".to_string()));
    let expected = "\
values:
- Red
- Green
description: A traffic-light hue.
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn canonical_one_of_serializes_as_tag_and_variants() {
    let schema = OneOfSchema::new(
        "kind",
        vec![OneOfVariant {
            wire_name: "Login".to_string(),
            inner_reference: "#/components/schemas/LoginData".to_string(),
        }],
    );
    let expected = "\
tag: kind
variants:
- wire_name: Login
  inner_reference: '#/components/schemas/LoginData'
";
    assert_eq!(render(&schema), expected);
}

#[test]
fn canonical_components_dump_round_trips_every_variant() {
    // A `Components` holding one schema of each `SchemaObject` variant
    // survives serialize -> deserialize through the canonical path:
    // the untagged deserializer recovers each variant from its
    // canonical shape (`tag`+`variants` -> OneOf, `values` ->
    // StringEnum, catch-all -> Object).
    let mut schemas: IndexMap<String, SchemaObject> = IndexMap::new();
    schemas.insert(
        "Event".to_string(),
        SchemaObject::OneOf(OneOfSchema::new(
            "kind",
            vec![OneOfVariant {
                wire_name: "Login".to_string(),
                inner_reference: "#/components/schemas/LoginData".to_string(),
            }],
        )),
    );
    schemas.insert(
        "Color".to_string(),
        SchemaObject::StringEnum(StringEnumSchema::new(vec![
            "Red".to_string(),
            "Green".to_string(),
        ])),
    );
    schemas.insert(
        "Wrapper".to_string(),
        SchemaObject::Object(ObjectSchema {
            reference: Some("#/components/schemas/Inner".to_string()),
            nullable: Some(true),
            ..ObjectSchema::empty()
        }),
    );
    let components = Components {
        schemas,
        other: BTreeMap::new(),
    };

    let json = serde_json::to_string(&components).expect("canonical JSON dump must succeed");
    let parsed: Components = serde_json::from_str(&json).expect("canonical dump must parse back");
    assert_eq!(parsed, components);

    let yaml_dump = render(&components);
    let parsed: Components =
        yaml::from_str(&yaml_dump).expect("canonical YAML dump must parse back");
    assert_eq!(parsed, components);
}
