//! `from_schemas` builds a complete `Document` from an `Info` value
//! and a `Schemas` collection, with no partial to merge against. The
//! resulting document is format-neutral — it can be rendered to YAML
//! or JSON without re-running the schema pipeline.
//!
//! The scratch-path counterpart to `compose`: when a caller has
//! Rust-generated schemas but no hand-written OAS partial, this is the
//! one-call entry point. The OAS version is an explicit argument and
//! is stamped onto the document as runtime data.

use std::collections::BTreeMap;

use frieze::Schema;
use frieze_openapi::{Info, Version};
use frieze_usecase::from_schemas;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

fn snapshot_info() -> Info {
    Info {
        title: "Generated API".to_string(),
        version: "1.2.3".to_string(),
        description: None,
        extensions: BTreeMap::new(),
    }
}

fn user_schemas() -> frieze_model::Schemas {
    frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input")
}

#[test]
fn from_schemas_routes_schemas_through_components() {
    let document = from_schemas(snapshot_info(), user_schemas(), Version::V3_0);

    // The document carries the supplied `Info` verbatim, and both
    // version fields reflect the requested `Version`.
    assert_eq!(document.info.title, "Generated API");
    assert_eq!(document.info.version, "1.2.3");
    assert_eq!(document.oas_version, Version::V3_0);
    assert_eq!(document.openapi, "3.0.3");

    // `components.schemas` is populated; sections the caller did not
    // supply (`paths`, `servers`, `tags`, vendor extensions) are
    // absent.
    let components = document
        .components
        .as_ref()
        .expect("from_schemas always sets components");
    assert!(components.schemas.contains_key("User"));
    assert_eq!(components.schemas.len(), 1);
    assert!(document.paths.is_none());
    assert!(document.servers.is_none());
    assert!(document.tags.is_none());
    assert!(document.extensions.is_empty());

    // Format-neutral: the same document round-trips through YAML and
    // JSON without loss.
    let yaml = frieze_openapi::to_yaml(&document);
    let from_yaml: frieze_openapi::Document =
        serde_yaml::from_str(&yaml).expect("YAML round-trip must succeed");
    let json = serde_json::to_string_pretty(&document).expect("JSON serialize must succeed");
    let from_json: frieze_openapi::Document =
        serde_json::from_str(&json).expect("JSON round-trip must succeed");
    assert_eq!(from_yaml, from_json);
}

#[test]
fn from_schemas_accepts_either_version_in_the_same_build() {
    // The version argument is runtime data — the same build serves
    // both supported versions, and the stamped `openapi` string
    // follows the argument.
    for (version, expected_openapi) in [(Version::V3_0, "3.0.3"), (Version::V3_1, "3.1.0")] {
        let document = from_schemas(snapshot_info(), user_schemas(), version);
        assert_eq!(document.oas_version, version);
        assert_eq!(document.openapi, expected_openapi);
    }
}
