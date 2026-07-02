//! `from_schemas` builds a complete `Document` from an `Info` value
//! and a `Schemas` collection, with no partial to merge against. The
//! resulting document is format-neutral — it can be rendered to YAML
//! or JSON without re-running the schema pipeline.
//!
//! The scratch-path counterpart to `compose`: when a caller has
//! Rust-generated schemas but no hand-written OAS partial, this is the
//! one-call entry point.

use std::collections::BTreeMap;

use frieze::{from_schemas, Info, Schema};

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

#[test]
fn from_schemas_routes_schemas_through_components() {
    let schemas: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    let document = from_schemas(snapshot_info(), schemas);

    // The document carries the supplied `Info` verbatim.
    assert_eq!(document.info.title, "Generated API");
    assert_eq!(document.info.version, "1.2.3");

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
    let yaml = frieze::to_yaml(&document);
    let from_yaml: frieze::Document =
        serde_yaml::from_str(&yaml).expect("YAML round-trip must succeed");
    let json = serde_json::to_string_pretty(&document).expect("JSON serialize must succeed");
    let from_json: frieze::Document =
        serde_json::from_str(&json).expect("JSON round-trip must succeed");
    assert_eq!(from_yaml, from_json);
}
