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
//! [`compose_components`] is the lower-level half of [`compose`] for
//! callers that already hold a canonical [`Components`] value and want
//! to merge it into one or more partials without re-running the
//! boundary conversion.
//!
//! Both paths funnel through the same boundary conversion
//! ([`crate::components_from_schemas`]) so the resulting
//! `components.schemas` map is identical for the same input.
//!
//! The OAS version is per-document runtime data: `compose` carries the
//! partial's version (lifted from its `openapi:` string at parse
//! time) into the output, and `from_schemas` stamps the explicitly
//! requested one. Serialization in `frieze-openapi` dispatches on the
//! document's `oas_version`, so a single build can compose and emit
//! 3.0 and 3.1 documents side by side.

use frieze_model::{Error, Schemas};
use frieze_openapi::{Components, Document, Info, Version};

use crate::boundary::components_from_schemas;

/// Merges `schemas` into `partial.components.schemas` and returns the
/// resulting complete OAS document.
///
/// `partial` MUST NOT already contain any entries under
/// `components.schemas`; the single source of truth for schemas is the
/// Rust types collected via `frieze::SchemasBuilder`. If
/// `partial.components.schemas` is non-empty, returns
/// [`Error::PartialAlreadyHasSchemas`].
///
/// Other parts of `partial` (`info`, `paths`, `servers`, `tags`, vendor
/// extensions, ...) are preserved verbatim — `compose` writes only the
/// `components.schemas` slot. That includes the `openapi` string: a
/// partial that says `openapi: 3.0.5` keeps that exact patch string in
/// the output, and the partial's `oas_version` (lifted at parse time)
/// determines the OAS shape the composed document serializes as.
///
/// Schemas are inserted in alphabetical order, matching the canonical
/// output order of [`Schemas`] (which is backed by a [`BTreeMap`]
/// keyed by [`frieze_model::SchemaName`]).
///
/// [`BTreeMap`]: std::collections::BTreeMap
pub fn compose(partial: Document, schemas: Schemas) -> Result<Document, Error> {
    compose_components(partial, components_from_schemas(&schemas))
}

/// Merges an already-converted [`Components`] value into
/// `partial.components.schemas` and returns the resulting complete OAS
/// document.
///
/// This is the second half of [`compose`]: [`compose`] converts a
/// [`Schemas`] collection through the boundary
/// ([`crate::components_from_schemas`]) and then delegates here. The
/// split exists for callers that already hold the canonical
/// [`Components`] form — e.g. a flow that collects components once and
/// composes them into several partial documents — so the conversion is
/// not repeated per document.
///
/// The same precondition as [`compose`] applies: `partial` must not
/// already contain entries under `components.schemas`
/// ([`Error::PartialAlreadyHasSchemas`] otherwise), and everything
/// else in `partial` is preserved verbatim. Only the `schemas` map of
/// `components` is consumed; a `Components` produced by
/// [`crate::components_from_schemas`] (or parsed back from its
/// canonical dump) carries nothing else.
pub fn compose_components(
    mut partial: Document,
    components: Components,
) -> Result<Document, Error> {
    let slot = partial.components.get_or_insert_with(Components::default);
    if !slot.schemas.is_empty() {
        return Err(Error::PartialAlreadyHasSchemas {
            count: slot.schemas.len(),
        });
    }
    slot.schemas = components.schemas;
    Ok(partial)
}

/// Builds a complete [`Document`] from an [`Info`] and a [`Schemas`]
/// collection, with no partial document to merge with.
///
/// The resulting document has empty `paths`, no `servers`, no `tags`,
/// etc.; the caller may attach those via direct field access if
/// needed. The document's `openapi` string is the canonical one for
/// the requested `version` ([`Version::openapi_string`]) and its
/// `oas_version` field is set to the same `version` — serialization
/// dispatches on it, so the returned document emits the OAS shape of
/// whichever version was asked for.
///
/// This is the scratch-path counterpart to [`compose`]: when the caller
/// has Rust-generated schemas but no hand-written partial OAS document,
/// `from_schemas` produces the same `components.schemas` layout that
/// `compose` would have written into a partial.
pub fn from_schemas(info: Info, schemas: Schemas, version: Version) -> Document {
    Document::from_components(info, components_from_schemas(&schemas), version)
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

    #[test]
    fn from_schemas_stamps_the_requested_version() {
        for version in [Version::V3_0, Version::V3_1] {
            let document = from_schemas(plain_info(), empty_schemas(), version);
            assert_eq!(document.oas_version, version);
            assert_eq!(document.openapi, version.openapi_string());
        }
    }

    #[test]
    fn compose_accepts_partials_of_both_versions() {
        for version in [Version::V3_0, Version::V3_1] {
            let partial = Document::from_components(plain_info(), Components::default(), version);
            let composed =
                compose(partial, empty_schemas()).expect("empty partial must compose cleanly");
            assert_eq!(composed.oas_version, version);
        }
    }

    #[test]
    fn compose_preserves_the_partial_openapi_string_verbatim() {
        // The partial's raw `openapi:` string (patch included) is what
        // the composed document carries — `compose` never rewrites it
        // to a canonical patch.
        let mut partial =
            Document::from_components(plain_info(), Components::default(), Version::V3_0);
        partial.openapi = "3.0.99".to_string();
        let composed = compose(partial, empty_schemas()).expect("compose must succeed");
        assert_eq!(composed.openapi, "3.0.99");
        assert_eq!(composed.oas_version, Version::V3_0);
    }
}
