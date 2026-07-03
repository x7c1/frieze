//! Test-only helpers shared across the integration tests.
//!
//! Placed under `tests/common/mod.rs` (rather than a top-level
//! `tests/common.rs`) so cargo does not pick it up as a separate test
//! binary — each test file opts in with `mod common;`. Not every test
//! binary uses every helper, hence the `#[allow(dead_code)]` on each.

use std::collections::BTreeMap;

use frieze_model::Schemas;
use frieze_openapi::{to_yaml, Document, Info, Version};
use frieze_usecase::components_from_schemas;

/// Renders a `Schemas` collection as a minimal `Document` under
/// **both** OAS versions, asserts the two renders are byte-identical
/// (modulo the version-dependent `openapi:` header, which both renders
/// normalize away), and returns the shared YAML.
///
/// This is the helper for the version-neutral snapshot tests: shapes
/// with no nullability and no `$ref`-sibling concerns must emit the
/// same bytes under OAS 3.0 and 3.1, and the built-in comparison keeps
/// every such test exercising both emitters in a single build.
///
/// Tests that pin a version-specific shape use [`snapshot_yaml_3_0`] /
/// [`snapshot_yaml_3_1`] instead.
#[allow(dead_code)]
pub fn snapshot_yaml(schemas: Schemas) -> String {
    let rendered_3_0 = render(&schemas, Version::V3_0);
    let rendered_3_1 = render(&schemas, Version::V3_1);
    assert_eq!(
        rendered_3_0, rendered_3_1,
        "snapshot_yaml is for version-neutral shapes; \
         this schema set renders differently under OAS 3.0 and 3.1 — \
         use snapshot_yaml_3_0 / snapshot_yaml_3_1 with per-version snapshots instead"
    );
    rendered_3_0
}

/// Renders a `Schemas` collection as a minimal `Document` targeting
/// OAS 3.0. For tests that pin a 3.0-specific shape (`nullable: true`,
/// the `allOf` wraps around `$ref`).
#[allow(dead_code)]
pub fn snapshot_yaml_3_0(schemas: Schemas) -> String {
    render(&schemas, Version::V3_0)
}

/// Renders a `Schemas` collection as a minimal `Document` targeting
/// OAS 3.1. For tests that pin a 3.1-specific shape (`type` sequences
/// with `"null"`, the `oneOf` null arm, `$ref` siblings).
#[allow(dead_code)]
pub fn snapshot_yaml_3_1(schemas: Schemas) -> String {
    render(&schemas, Version::V3_1)
}

/// Wraps a `Schemas` collection in a minimal `Document` for snapshot
/// tests, then renders it to YAML.
///
/// The wrapping `Info` uses fixed `title` / `version` values so the
/// snapshot prefix is byte-identical across every snapshot — the only
/// per-test variation in the output is the `components.schemas`
/// content.
///
/// The `openapi` version field is overwritten with a fixed placeholder
/// (`"X.Y.Z"`) so the same inline snapshot literal works regardless of
/// the version the document was built for (`3.0.3` / `3.1.0`). Note
/// that only the header string is normalized — the *shape* dispatch
/// still follows the document's `oas_version`. Tests that need to
/// assert the version string should do so directly via the document
/// field, not through this shared snapshot path.
fn render(schemas: &Schemas, version: Version) -> String {
    let mut document =
        Document::from_components(snapshot_info(), components_from_schemas(schemas), version);
    document.openapi = "X.Y.Z".to_string();
    to_yaml(&document)
}

/// Fixed `Info` value used by every snapshot test so that the leading
/// `info: { title, version }` block is constant.
fn snapshot_info() -> Info {
    Info {
        title: "snapshot test".to_string(),
        version: "0.0.0".to_string(),
        description: None,
        extensions: BTreeMap::new(),
    }
}
