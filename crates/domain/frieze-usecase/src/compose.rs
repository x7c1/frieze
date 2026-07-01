//! Composing a complete [`OasDocument`] from a hand-written partial and
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
//!   `OasDocument` from an `Info` value and a `Schemas`, with empty
//!   `paths` / `servers` / etc. for callers that have no partial to
//!   merge with.
//!
//! Both paths funnel through the same boundary conversion
//! ([`crate::boundary::to_openapi`]) so the resulting `components.schemas`
//! map is identical for the same input.
//!
//! # Transition guard
//!
//! Both entry points reject an [`OasVersion`] that does not match the
//! version this crate was compiled for. The `Serialize` implementations
//! in `frieze-openapi` are still selected at compile time by the
//! `oas-3-0` / `oas-3-1` features; letting a caller ask for the
//! "wrong" version would produce a document whose header (`openapi:
//! 3.1.0`) disagrees with the shape of the body (3.0-style
//! `nullable: true`, etc.). The guard fails fast with
//! [`Error::UnsupportedOasVersion`] instead. It will be removed once
//! `Serialize` becomes runtime-dispatched on `oas_version`.

use frieze_model::{Error, Schemas};
use frieze_openapi::{Components, Info, OasDocument, OasVersion};

use crate::boundary::to_openapi;

/// The [`OasVersion`] this crate was compiled to emit.
///
/// The `Serialize` implementations for the `frieze-openapi` schema
/// object family are still selected by the `oas-3-0` / `oas-3-1`
/// features at compile time; this function is the single source of
/// truth for the runtime-visible "which version am I" answer while the
/// serializers remain cfg-gated. Used by [`from_schemas`] and
/// [`compose`] for the transition guard.
#[cfg(feature = "oas-3-0")]
fn active_oas_version() -> OasVersion {
    OasVersion::V3_0
}

#[cfg(feature = "oas-3-1")]
fn active_oas_version() -> OasVersion {
    OasVersion::V3_1
}

/// Merges `schemas` into `partial.components.schemas` and returns the
/// resulting complete OAS document.
///
/// `partial` MUST NOT already contain any entries under
/// `components.schemas`; the single source of truth for schemas is the
/// Rust types collected via [`crate::SchemasBuilder`]. If
/// `partial.components.schemas` is non-empty, returns
/// [`Error::PartialAlreadyHasSchemas`].
///
/// The partial's `oas_version` (lifted from its `openapi:` string at
/// parse time) must match the version this crate was compiled for. A
/// mismatch returns [`Error::UnsupportedOasVersion`] — see the module
/// docs for the rationale.
///
/// Other parts of `partial` (`info`, `paths`, `servers`, `tags`, vendor
/// extensions, ...) are preserved verbatim — `compose` writes only the
/// `components.schemas` slot.
///
/// Schemas are inserted in alphabetical order, matching the canonical
/// output order of [`Schemas`] (which is backed by a [`BTreeMap`]
/// keyed by [`frieze_model::SchemaName`]).
///
/// [`BTreeMap`]: std::collections::BTreeMap
pub fn compose(mut partial: OasDocument, schemas: Schemas) -> Result<OasDocument, Error> {
    let active = active_oas_version();
    if partial.oas_version != active {
        return Err(Error::UnsupportedOasVersion {
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

/// Builds a complete [`OasDocument`] from an [`Info`] and a [`Schemas`]
/// collection, with no partial document to merge with.
///
/// The resulting document has empty `paths`, no `servers`, no `tags`,
/// etc.; the caller may attach those via direct field access if
/// needed. The `openapi` patch string is taken from
/// [`OasVersion::openapi_string`] for the requested `version`, and the
/// same `version` is stored on `oas_version`.
///
/// `version` must match the OAS version this crate was compiled for.
/// A mismatch returns [`Error::UnsupportedOasVersion`] — see the
/// module docs for the rationale.
///
/// This is the scratch-path counterpart to [`compose`]: when the
/// caller has Rust-generated schemas but no hand-written partial OAS
/// document, `from_schemas` produces the same `components.schemas`
/// layout that `compose` would have written into a partial.
pub fn from_schemas(
    info: Info,
    schemas: Schemas,
    version: OasVersion,
) -> Result<OasDocument, Error> {
    let active = active_oas_version();
    if version != active {
        return Err(Error::UnsupportedOasVersion {
            got: version.openapi_string().to_string(),
        });
    }
    let mut components = Components::default();
    for (name, schema) in schemas.by_name {
        if let Some(object) = to_openapi(&schema) {
            components.schemas.insert(name.into_string(), object);
        }
    }
    let mut document = OasDocument::from_components(info, components);
    // `from_components` sets `openapi` and `oas_version` from the
    // active-feature default. Overwriting here makes the "caller-requested
    // version wins" intent explicit and eases a later refactor when the
    // guard above is dropped.
    document.openapi = version.openapi_string().to_string();
    document.oas_version = version;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn empty_schemas() -> Schemas {
        Schemas::new(Vec::new()).unwrap()
    }

    fn plain_info() -> Info {
        Info {
            title: "t".to_string(),
            version: "v".to_string(),
            description: None,
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    #[cfg(feature = "oas-3-0")]
    fn from_schemas_accepts_matching_version_3_0() {
        let document = from_schemas(plain_info(), empty_schemas(), OasVersion::V3_0)
            .expect("matching version must succeed");
        assert_eq!(document.oas_version, OasVersion::V3_0);
        assert_eq!(document.openapi, "3.0.3");
    }

    #[test]
    #[cfg(feature = "oas-3-0")]
    fn from_schemas_rejects_mismatched_version_3_1() {
        let err = from_schemas(plain_info(), empty_schemas(), OasVersion::V3_1).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOasVersion {
                got: "3.1.0".to_string(),
            }
        );
    }

    #[test]
    #[cfg(feature = "oas-3-1")]
    fn from_schemas_accepts_matching_version_3_1() {
        let document = from_schemas(plain_info(), empty_schemas(), OasVersion::V3_1)
            .expect("matching version must succeed");
        assert_eq!(document.oas_version, OasVersion::V3_1);
        assert_eq!(document.openapi, "3.1.0");
    }

    #[test]
    #[cfg(feature = "oas-3-1")]
    fn from_schemas_rejects_mismatched_version_3_0() {
        let err = from_schemas(plain_info(), empty_schemas(), OasVersion::V3_0).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOasVersion {
                got: "3.0.3".to_string(),
            }
        );
    }

    #[test]
    #[cfg(feature = "oas-3-0")]
    fn compose_accepts_matching_partial_version_3_0() {
        let mut partial = OasDocument::from_components(plain_info(), Components::default());
        // Clear the components so we exercise the fill path from an
        // empty partial. (`from_components` sets `Some(Components)`, but
        // its `schemas` map is empty.)
        assert_eq!(partial.oas_version, OasVersion::V3_0);
        partial.components = None;
        let result = compose(partial, empty_schemas()).expect("matching version must succeed");
        assert_eq!(result.oas_version, OasVersion::V3_0);
    }

    #[test]
    #[cfg(feature = "oas-3-0")]
    fn compose_rejects_mismatched_partial_version_3_1() {
        let mut partial = OasDocument::from_components(plain_info(), Components::default());
        partial.openapi = "3.1.0".to_string();
        partial.oas_version = OasVersion::V3_1;
        let err = compose(partial, empty_schemas()).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOasVersion {
                got: "3.1.0".to_string(),
            }
        );
    }

    #[test]
    #[cfg(feature = "oas-3-1")]
    fn compose_accepts_matching_partial_version_3_1() {
        let partial = OasDocument::from_components(plain_info(), Components::default());
        assert_eq!(partial.oas_version, OasVersion::V3_1);
        let mut p = partial;
        p.components = None;
        let result = compose(p, empty_schemas()).expect("matching version must succeed");
        assert_eq!(result.oas_version, OasVersion::V3_1);
    }

    #[test]
    #[cfg(feature = "oas-3-1")]
    fn compose_rejects_mismatched_partial_version_3_0() {
        let mut partial = OasDocument::from_components(plain_info(), Components::default());
        partial.openapi = "3.0.3".to_string();
        partial.oas_version = OasVersion::V3_0;
        let err = compose(partial, empty_schemas()).unwrap_err();
        assert_eq!(
            err,
            Error::UnsupportedOasVersion {
                got: "3.0.3".to_string(),
            }
        );
    }
}
