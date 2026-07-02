//! The OpenAPI document root.
//!
//! [`Document`] is the top-level type that carries an entire OpenAPI
//! specification in memory. It is the format-neutral hand-off between
//! the schema-producing side of frieze (which populates
//! `components.schemas`) and the eventual output renderers — the same
//! value can be serialized to YAML or JSON without re-running the
//! schema pipeline.
//!
//! Only the keys frieze currently constructs or routinely needs to
//! preserve on the wire are modelled with dedicated fields. Everything
//! else (`paths`, `servers`, `security`, `tags`, `externalDocs`, and any
//! vendor extension `x-*` or future top-level key) is held as opaque
//! [`serde_json::Value`] so a round-trip through this struct does not
//! lose user-supplied data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::components::Components;
use crate::info::Info;

/// An OpenAPI document.
///
/// The struct is intentionally tolerant: fields frieze does not produce
/// itself are typed as `Option<serde_json::Value>` and carried through
/// untouched, so callers can parse an existing document, mutate
/// `components.schemas`, and serialize the result without losing the
/// other sections.
///
/// `extensions` (the `#[serde(flatten)]` catch-all) absorbs vendor
/// extensions (`x-*`) and any top-level key not yet modelled. It uses
/// [`BTreeMap`] so the ordering of those keys on the wire is
/// deterministic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// The OAS version string (e.g. `"3.0.3"`, `"3.1.0"`).
    pub openapi: String,
    pub info: Info,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servers: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "externalDocs"
    )]
    pub external_docs: Option<serde_json::Value>,
    /// Vendor extensions (`x-*`) and any other top-level field not yet
    /// modelled. Round-trips verbatim.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

impl Document {
    /// Constructs a complete [`Document`] from an [`Info`] and a
    /// [`Components`] whose `schemas` map has been pre-populated.
    ///
    /// The resulting document has empty `paths`, no `servers`, etc.; the
    /// caller adds those via direct field access if needed. The OAS
    /// version string is taken from the active feature flag
    /// (`oas-3-0` → `"3.0.3"`, `oas-3-1` → `"3.1.0"`).
    ///
    /// This constructor is format-neutral — the returned value can be
    /// rendered to YAML (via `serde_yaml::to_string`) or JSON (via
    /// `serde_json::to_string`) without re-running the schema pipeline.
    pub fn from_components(info: Info, components: Components) -> Self {
        Self {
            openapi: oas_version_string().to_string(),
            info,
            servers: None,
            paths: None,
            components: Some(components),
            security: None,
            tags: None,
            external_docs: None,
            extensions: BTreeMap::new(),
        }
    }
}

/// The OAS version string for the active feature flag.
#[cfg(feature = "oas-3-0")]
fn oas_version_string() -> &'static str {
    "3.0.3"
}

#[cfg(feature = "oas-3-1")]
fn oas_version_string() -> &'static str {
    "3.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_schema::ObjectSchema;
    use crate::schema_object::SchemaObject;
    use crate::schema_type::SchemaType;
    use indexmap::IndexMap;

    fn sample_yaml() -> &'static str {
        // A small document that exercises:
        // - the required top-level fields (`openapi`, `info`)
        // - a modelled-but-opaque field (`paths`)
        // - the explicitly-modelled `components` (with an empty schemas
        //   map, since populating it with frieze-built schemas is a
        //   later step)
        // - a vendor extension at the top level (`x-codegen-info`)
        // - a vendor extension inside `info`
        // - the opaque `tags` field
        "openapi: 3.0.3\n\
         info:\n  \
           title: Example API\n  \
           version: 1.0.0\n  \
           description: An example API.\n  \
           x-internal-id: 42\n\
         paths:\n  \
           /ping:\n    \
             get:\n      \
               summary: ping\n\
         components:\n  \
           schemas: {}\n\
         tags:\n  \
           - name: example\n\
         x-codegen-info:\n  \
           generator: frieze\n"
    }

    #[test]
    fn round_trips_through_yaml() {
        let yaml = sample_yaml();
        let first: Document = serde_yaml::from_str(yaml).expect("first parse must succeed");
        let reserialized =
            serde_yaml::to_string(&first).expect("serializing back to YAML must succeed");
        let second: Document =
            serde_yaml::from_str(&reserialized).expect("second parse must succeed");
        assert_eq!(first, second);
    }

    #[test]
    fn round_trips_through_json() {
        // Same source, but routed through JSON: parse YAML once, then
        // serialize to JSON, parse the JSON, and compare.
        let yaml = sample_yaml();
        let first: Document = serde_yaml::from_str(yaml).expect("first parse must succeed");
        let json = serde_json::to_string_pretty(&first).expect("serializing to JSON must succeed");
        let second: Document =
            serde_json::from_str(&json).expect("parsing back from JSON must succeed");
        assert_eq!(first, second);
    }

    #[test]
    fn programmatically_constructed_document_round_trips() {
        // Build a document by hand (no parse step) to catch any serde
        // configuration mistake that breaks construct-then-serialize.
        let mut schemas: IndexMap<String, SchemaObject> = IndexMap::new();
        schemas.insert(
            "Empty".to_string(),
            SchemaObject::Object(ObjectSchema {
                ty: Some(SchemaType::Object),
                ..ObjectSchema::default()
            }),
        );

        let document = Document {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "Programmatic".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                extensions: BTreeMap::new(),
            },
            servers: None,
            paths: None,
            components: Some(Components {
                schemas,
                other: BTreeMap::new(),
            }),
            security: None,
            tags: None,
            external_docs: None,
            extensions: BTreeMap::new(),
        };

        let yaml = serde_yaml::to_string(&document)
            .expect("serializing constructed document must succeed");
        let parsed: Document =
            serde_yaml::from_str(&yaml).expect("parsing back constructed document must succeed");
        assert_eq!(document, parsed);
    }
}
