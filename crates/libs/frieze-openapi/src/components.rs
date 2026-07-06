//! The `components` block of an OpenAPI document.
//!
//! The OAS specification places several reusable maps under
//! `#/components` (`schemas`, `responses`, `parameters`, `examples`,
//! `requestBodies`, `headers`, `securitySchemes`, `links`, `callbacks`,
//! `pathItems`). frieze currently only constructs entries under
//! `schemas`, so that map is the one explicitly modelled here; any other
//! component map present on the wire is captured as opaque JSON in
//! [`Components::other`] so it survives a round-trip without loss.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::schema_object::SchemaObject;

/// The OAS `components` object.
///
/// `schemas` uses [`IndexMap`] because the order in which schema entries
/// are emitted is part of frieze's contract (insertion order, which
/// today is registration order from `SchemasBuilder`). Other component
/// maps that frieze does not yet construct are captured as
/// [`serde_json::Value`] in `other`; a [`BTreeMap`] is used there for the
/// same alphabetical-ordering reason as in [`crate::Info::extensions`].
///
/// # Canonical versus OAS wire form
///
/// The derived `Serialize` here is the *canonical, version-neutral*
/// form: each schema entry serializes through its own derived impl
/// (`$ref` / `description` / `nullable` as plain siblings, string
/// enums as `{values}`, ...), and the output round-trips through the
/// derived `Deserialize`. Serialize a `Components` directly (e.g.
/// `serde_json::to_writer(w, &components)`) to produce a
/// machine-readable dump that a separate process can parse back.
///
/// The *OAS wire form* is emitted when a `Components` rides inside a
/// serialized [`crate::Document`]: the document routes it through the
/// crate-private versioned emitter keyed by its `oas_version`, which
/// applies the OAS 3.0 / 3.1 encoding split.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Components {
    /// Named schema definitions referenced by `$ref` from elsewhere in
    /// the document. Insertion order is preserved on the wire.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub schemas: IndexMap<String, SchemaObject>,
    /// Any other key under `components` (e.g. `responses`, `parameters`,
    /// `securitySchemes`) that frieze does not model. Round-trips
    /// verbatim.
    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}
