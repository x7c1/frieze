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
