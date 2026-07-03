//! Test-only helpers shared across the integration tests.
//!
//! Placed under `tests/common/mod.rs` (rather than a top-level
//! `tests/common.rs`) so cargo does not pick it up as a separate test
//! binary — each test file opts in with `mod common;`.

use std::collections::BTreeMap;

use frieze_model::Schemas;
use frieze_openapi::{to_yaml, Info, Version};
use frieze_usecase::from_schemas;

/// Wraps a `Schemas` collection in a minimal `Document` for snapshot
/// tests, then renders it to YAML.
///
/// The wrapping `Info` uses fixed `title` / `version` values so the
/// snapshot prefix is byte-identical across every snapshot — the only
/// per-test variation in the output is the `components.schemas`
/// content.
///
/// The `openapi` version field is also overwritten with a fixed
/// placeholder (`"X.Y.Z"`) so the same inline snapshot literal can
/// match under both `oas-3-0` (which would otherwise emit `3.0.3`) and
/// `oas-3-1` (`3.1.0`). Tests that need to assert the version string
/// should do so directly via the document field, not through this
/// shared snapshot path.
pub fn snapshot_yaml(schemas: Schemas) -> String {
    let mut document = from_schemas(snapshot_info(), schemas, compiled_version())
        .expect("from_schemas with the compiled-in OAS version must succeed");
    document.openapi = "X.Y.Z".to_string();
    to_yaml(&document)
}

/// The OAS version the current build was compiled to emit.
///
/// `from_schemas` (temporarily) rejects any other version while the
/// `Serialize` impls remain feature-gated, so the snapshot helper asks
/// for exactly this one instead of hard-coding a variant per test.
#[cfg(feature = "oas-3-0")]
fn compiled_version() -> Version {
    Version::V3_0
}

#[cfg(feature = "oas-3-1")]
fn compiled_version() -> Version {
    Version::V3_1
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
