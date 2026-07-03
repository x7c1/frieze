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
use crate::version::Version;

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
///
/// # `openapi` versus `oas_version`
///
/// - `openapi` is the verbatim version string as it appears on the
///   wire (e.g. `"3.0.3"`, `"3.0.99"`, `"3.1.0"`); it is what gets
///   serialized back out.
/// - `oas_version` is the major.minor discriminant lifted from that
///   string at deserialize time via [`Version::parse_from_openapi`].
///   It never appears on the wire and exists purely as a runtime
///   handle for OAS-shape dispatch.
///
/// Deserializing a document whose `openapi` field is missing, empty,
/// or outside the supported range (`3.0.x` / `3.1.x`) fails with a
/// deserialize error carrying the [`crate::VersionParseError`]
/// message. Callers that construct a `Document` through struct-literal
/// syntax are responsible for keeping the two fields consistent; the
/// library's own constructor ([`Document::from_components`]) does so
/// automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DocumentRaw")]
pub struct Document {
    /// The OAS version string (e.g. `"3.0.3"`, `"3.1.0"`).
    pub openapi: String,
    /// Major.minor discriminant lifted from [`Self::openapi`] at
    /// deserialize time. Not serialized — the wire format only carries
    /// the `openapi` string.
    #[serde(skip)]
    pub oas_version: Version,
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
    /// caller adds those via direct field access if needed. The
    /// `openapi` string is the canonical one for the requested
    /// `version` ([`Version::openapi_string`]), and the same `version`
    /// is stored in `oas_version` — the two fields start out
    /// consistent by construction.
    ///
    /// This constructor is format-neutral — the returned value can be
    /// rendered to YAML (via `serde_yaml::to_string`) or JSON (via
    /// `serde_json::to_string`) without re-running the schema pipeline.
    pub fn from_components(info: Info, components: Components, version: Version) -> Self {
        Self {
            openapi: version.openapi_string().to_string(),
            oas_version: version,
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

/// Wire-shape twin of [`Document`], used only on the deserialize path.
///
/// Mirrors the same field layout so serde can decode a YAML/JSON
/// document; the [`TryFrom`] impl below then promotes the raw form to
/// a [`Document`], parsing the `openapi` string into a [`Version`]
/// along the way. Keeping the twin private isolates the lifting logic
/// from the derived `Serialize` side, which keeps emitting only wire
/// fields (`oas_version` is skipped).
#[derive(Deserialize)]
struct DocumentRaw {
    openapi: String,
    info: Info,
    #[serde(default)]
    servers: Option<serde_json::Value>,
    #[serde(default)]
    paths: Option<serde_json::Value>,
    #[serde(default)]
    components: Option<Components>,
    #[serde(default)]
    security: Option<serde_json::Value>,
    #[serde(default)]
    tags: Option<serde_json::Value>,
    #[serde(default, rename = "externalDocs")]
    external_docs: Option<serde_json::Value>,
    #[serde(flatten)]
    extensions: BTreeMap<String, serde_json::Value>,
}

impl TryFrom<DocumentRaw> for Document {
    type Error = crate::version::VersionParseError;

    fn try_from(raw: DocumentRaw) -> Result<Self, Self::Error> {
        let oas_version = Version::parse_from_openapi(&raw.openapi)?;
        Ok(Self {
            openapi: raw.openapi,
            oas_version,
            info: raw.info,
            servers: raw.servers,
            paths: raw.paths,
            components: raw.components,
            security: raw.security,
            tags: raw.tags,
            external_docs: raw.external_docs,
            extensions: raw.extensions,
        })
    }
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
            oas_version: Version::V3_0,
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

    #[test]
    fn deserialize_lifts_oas_version_3_0() {
        let yaml = "openapi: 3.0.3\ninfo:\n  title: t\n  version: v\n";
        let doc: Document = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(doc.oas_version, Version::V3_0);
        assert_eq!(doc.openapi, "3.0.3");
    }

    #[test]
    fn deserialize_lifts_oas_version_3_1() {
        let yaml = "openapi: 3.1.0\ninfo:\n  title: t\n  version: v\n";
        let doc: Document = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(doc.oas_version, Version::V3_1);
        assert_eq!(doc.openapi, "3.1.0");
    }

    #[test]
    fn deserialize_preserves_arbitrary_patch_verbatim() {
        // A hypothetical 3.0.99 patch release: the lifted discriminant
        // is V3_0, but the wire string is preserved verbatim.
        let yaml = "openapi: 3.0.99\ninfo:\n  title: t\n  version: v\n";
        let doc: Document = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(doc.oas_version, Version::V3_0);
        assert_eq!(doc.openapi, "3.0.99");
    }

    #[test]
    fn missing_openapi_field_fails_deserialize() {
        let yaml = "info:\n  title: t\n  version: v\n";
        let err = serde_yaml::from_str::<Document>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("openapi"),
            "expected error to mention `openapi`, got: {msg}"
        );
    }

    #[test]
    fn empty_openapi_field_fails_deserialize() {
        let yaml = "openapi: \"\"\ninfo:\n  title: t\n  version: v\n";
        let err = serde_yaml::from_str::<Document>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("empty"),
            "expected error to mention `empty`, got: {msg}"
        );
    }

    #[test]
    fn unsupported_openapi_field_fails_deserialize() {
        let yaml = "openapi: \"2.0\"\ninfo:\n  title: t\n  version: v\n";
        let err = serde_yaml::from_str::<Document>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported OAS version `2.0`"),
            "expected the unsupported-version message, got: {msg}"
        );
    }

    #[test]
    fn oas_version_never_appears_in_serialized_output() {
        let document = Document::from_components(
            Info {
                title: "t".to_string(),
                version: "v".to_string(),
                description: None,
                extensions: BTreeMap::new(),
            },
            Components::default(),
            Version::V3_0,
        );
        let yaml = serde_yaml::to_string(&document).unwrap();
        assert!(
            !yaml.contains("oas_version"),
            "yaml unexpectedly contains `oas_version`: {yaml}"
        );
        let json = serde_json::to_string(&document).unwrap();
        assert!(
            !json.contains("oas_version"),
            "json unexpectedly contains `oas_version`: {json}"
        );
    }

    #[test]
    fn from_components_keeps_openapi_and_oas_version_consistent() {
        let info = Info {
            title: "t".to_string(),
            version: "v".to_string(),
            description: None,
            extensions: BTreeMap::new(),
        };
        let doc = Document::from_components(info.clone(), Components::default(), Version::V3_0);
        assert_eq!(doc.openapi, "3.0.3");
        assert_eq!(doc.oas_version, Version::V3_0);

        let doc = Document::from_components(info, Components::default(), Version::V3_1);
        assert_eq!(doc.openapi, "3.1.0");
        assert_eq!(doc.oas_version, Version::V3_1);
    }
}
