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

use frieze_model::{Error, Schemas};
use frieze_openapi::{Components, Info, OasDocument};

use crate::boundary::to_openapi;

/// Merges `schemas` into `partial.components.schemas` and returns the
/// resulting complete OAS document.
///
/// `partial` MUST NOT already contain any entries under
/// `components.schemas`; the single source of truth for schemas is the
/// Rust types collected via [`crate::SchemasBuilder`]. If
/// `partial.components.schemas` is non-empty, returns
/// [`Error::PartialAlreadyHasSchemas`].
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
/// etc.; the caller may attach those via direct field access if needed.
/// The OAS version string is taken from the active feature flag
/// (`oas-3-0` → `"3.0.3"`, `oas-3-1` → `"3.1.0"`) by
/// [`OasDocument::from_components`].
///
/// This is the scratch-path counterpart to [`compose`]: when the caller
/// has Rust-generated schemas but no hand-written partial OAS document,
/// `from_schemas` produces the same `components.schemas` layout that
/// `compose` would have written into a partial.
pub fn from_schemas(info: Info, schemas: Schemas) -> OasDocument {
    let mut components = Components::default();
    for (name, schema) in schemas.by_name {
        if let Some(object) = to_openapi(&schema) {
            components.schemas.insert(name.into_string(), object);
        }
    }
    OasDocument::from_components(info, components)
}
