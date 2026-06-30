//! The `info` block of an OpenAPI document.
//!
//! Carries the metadata that the spec marks as required at the root
//! (`title`, `version`) plus the optional `description`. Anything else
//! the spec allows under `info` — including the vendor-extension `x-*`
//! keys — is captured in [`Info::extensions`] so a round-trip through
//! this struct does not lose user-supplied data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The OAS `info` object.
///
/// The struct intentionally models only the fields frieze currently needs
/// to construct programmatically. Other keys defined by the OAS
/// specification (`termsOfService`, `contact`, `license`, `summary`) and
/// any vendor extension (`x-*`) are captured by the catch-all
/// [`Info::extensions`] map so the value still round-trips verbatim
/// through serialize/deserialize.
///
/// `extensions` is a [`BTreeMap`] so that the on-the-wire ordering of
/// any keys that flow through it is deterministic (alphabetical) — this
/// keeps diffs of generated documents stable without requiring callers
/// to track insertion order for fields they did not set themselves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Vendor extensions (`x-*`) and any other top-level `info` key not
    /// modelled explicitly. Captured verbatim so unknown input survives
    /// a round-trip through this struct.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

impl Default for Info {
    /// Returns an `Info` with placeholder `title` / `version`, no
    /// description, and no extensions. Useful when constructing a
    /// document programmatically in tests or when filling in metadata
    /// progressively from another source.
    fn default() -> Self {
        Self {
            title: String::new(),
            version: String::new(),
            description: None,
            extensions: BTreeMap::new(),
        }
    }
}
