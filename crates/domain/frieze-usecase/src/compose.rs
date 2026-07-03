//! Composing a complete [`Document`] from a hand-written partial and
//! the Rust-generated [`Schemas`] collection.
//!
//! Two entry points cover the typical user flows:
//!
//! - [`compose`] merges schemas into an existing partial OAS document
//!   parsed from YAML or JSON. The partial carries `info`, `paths`,
//!   `tags`, vendor extensions, etc.; this side fills only
//!   `components.schemas`. Other parts of the partial are preserved
//!   verbatim.
//! - [`from_schemas`] is the scratch path — it builds a complete
//!   `Document` from an `Info` value, a `Schemas`, and a target
//!   [`Version`], with empty `paths` / `servers` / etc. for callers
//!   that have no partial to merge with.
//!
//! Both paths funnel through the same boundary conversion
//! ([`crate::boundary::to_openapi`]) so the resulting `components.schemas`
//! map is identical for the same input.
//!
//! # Transition guard
//!
//! Both entry points reject a [`Version`] that does not match the OAS
//! version this crate was compiled for. The `Serialize`
//! implementations in `frieze-openapi` are still selected at compile
//! time by the `oas-3-0` / `oas-3-1` features; letting a caller ask
//! for the other version would produce a document whose header
//! (`openapi: 3.1.0`) disagrees with the shape of its body (3.0-style
//! `nullable: true`, and so on). The guard fails fast with
//! [`Error::UnsupportedOpenApiVersion`] instead.
//!
//! The guard is temporary: it will be removed once `Serialize`
//! dispatches on the document's `oas_version` at runtime, at which
//! point a single build can emit both versions.

use frieze_model::{Error, Schemas};
use frieze_openapi::{Components, Document, Info, Version};

use crate::boundary::to_openapi;

/// The [`Version`] this crate was compiled to emit.
///
/// The `Serialize` implementations for the `frieze-openapi` schema
/// object family are still selected by the `oas-3-0` / `oas-3-1`
/// features at compile time; this function is the single runtime
/// answer to "which version is this build". Used by [`compose`] and
/// [`from_schemas`] for the transition guard described in the module
/// docs; it disappears together with that guard once `Serialize`
/// becomes runtime-dispatched.
#[cfg(feature = "oas-3-0")]
fn compiled_version() -> Version {
    Version::V3_0
}

#[cfg(feature = "oas-3-1")]
fn compiled_version() -> Version {
    Version::V3_1
}

/// Merges `schemas` into `partial.components.schemas` and returns the
/// resulting complete OAS document.
///
/// `partial` MUST NOT already contain any entries under
/// `components.schemas`; the single source of truth for schemas is the
/// Rust types collected via `frieze::SchemasBuilder`. If
/// `partial.components.schemas` is non-empty, returns
/// [`Error::PartialAlreadyHasSchemas`].
///
/// The partial's `oas_version` (lifted from its `openapi:` string at
/// parse time) must match the OAS version this crate was compiled
/// for; a mismatch returns [`Error::UnsupportedOpenApiVersion`] — see
/// the module docs for why this guard is (temporarily) in place.
///
/// Other parts of `partial` (`info`, `paths`, `servers`, `tags`, vendor
/// extensions, ...) are preserved verbatim — `compose` writes only the
/// `components.schemas` slot. That includes the `openapi` string: a
/// partial that says `openapi: 3.0.5` keeps that exact patch string in
/// the output.
///
/// Schemas are inserted in alphabetical order, matching the canonical
/// output order of [`Schemas`] (which is backed by a [`BTreeMap`]
/// keyed by [`frieze_model::SchemaName`]).
///
/// [`BTreeMap`]: std::collections::BTreeMap
pub fn compose(mut partial: Document, schemas: Schemas) -> Result<Document, Error> {
    if partial.oas_version != compiled_version() {
        return Err(Error::UnsupportedOpenApiVersion {
            got: partial.openapi.clone(),
        });
    }
    let components = partial.components.get_or_insert_with(Components::default);
    if !components.schemas.is_empty() {
        return Err(Error::PartialAlreadyHasSchemas {
            count: components.schemas.len(),
        });
    }
    for (name, schema) in schemas.by_name {
        if let Some(object) = to_openapi(&schema) {
            components.schemas.insert(name.into_string(), object);
        }
    }
    Ok(partial)
}

/// Builds a complete [`Document`] from an [`Info`] and a [`Schemas`]
/// collection, with no partial document to merge with.
///
/// The resulting document has empty `paths`, no `servers`, no `tags`,
/// etc.; the caller may attach those via direct field access if
/// needed. The document's `openapi` string is the canonical one for
/// the requested `version` ([`Version::openapi_string`]) and its
/// `oas_version` field is set to the same `version`.
///
/// `version` must match the OAS version this crate was compiled for;
/// a mismatch returns [`Error::UnsupportedOpenApiVersion`] — see the
/// module docs for why this guard is (temporarily) in place.
///
/// This is the scratch-path counterpart to [`compose`]: when the caller
/// has Rust-generated schemas but no hand-written partial OAS document,
/// `from_schemas` produces the same `components.schemas` layout that
/// `compose` would have written into a partial.
pub fn from_schemas(info: Info, schemas: Schemas, version: Version) -> Result<Document, Error> {
    if version != compiled_version() {
        return Err(Error::UnsupportedOpenApiVersion {
            got: version.openapi_string().to_string(),
        });
    }
    let mut components = Components::default();
    for (name, schema) in schemas.by_name {
        if let Some(object) = to_openapi(&schema) {
            components.schemas.insert(name.into_string(), object);
        }
    }
    Ok(Document::from_components(info, components, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn empty_schemas() -> Schemas {
        Schemas::new(Vec::new()).expect("empty schemas set is trivially valid")
    }

    fn plain_info() -> Info {
        Info {
            title: "t".to_string(),
            version: "v".to_string(),
            description: None,
            extensions: BTreeMap::new(),
        }
    }

    fn other_version() -> Version {
        match compiled_version() {
            Version::V3_0 => Version::V3_1,
            Version::V3_1 => Version::V3_0,
        }
    }

    #[test]
    fn from_schemas_accepts_the_compiled_version() {
        let version = compiled_version();
        let document = from_schemas(plain_info(), empty_schemas(), version)
            .expect("matching version must pass the transition guard");
        assert_eq!(document.oas_version, version);
        assert_eq!(document.openapi, version.openapi_string());
    }

    #[test]
    fn from_schemas_rejects_the_other_version() {
        let err = from_schemas(plain_info(), empty_schemas(), other_version()).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOpenApiVersion {
                got: other_version().openapi_string().to_string(),
            }
        );
    }

    #[test]
    fn compose_accepts_a_partial_of_the_compiled_version() {
        let partial =
            Document::from_components(plain_info(), Components::default(), compiled_version());
        let composed =
            compose(partial, empty_schemas()).expect("matching version must pass the guard");
        assert_eq!(composed.oas_version, compiled_version());
    }

    #[test]
    fn compose_rejects_a_partial_of_the_other_version() {
        let partial =
            Document::from_components(plain_info(), Components::default(), other_version());
        let err = compose(partial, empty_schemas()).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOpenApiVersion {
                got: other_version().openapi_string().to_string(),
            }
        );
    }

    #[test]
    fn compose_reports_the_partial_openapi_string_verbatim_on_mismatch() {
        // The guard error carries the partial's raw `openapi:` string
        // (patch included), not a canonicalized form.
        let mut partial =
            Document::from_components(plain_info(), Components::default(), other_version());
        let raw = match partial.oas_version {
            Version::V3_0 => "3.0.99",
            Version::V3_1 => "3.1.99",
        };
        partial.openapi = raw.to_string();
        let err = compose(partial, empty_schemas()).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOpenApiVersion {
                got: raw.to_string(),
            }
        );
    }
}
