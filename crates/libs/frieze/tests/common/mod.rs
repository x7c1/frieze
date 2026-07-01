//! Test-only helpers shared across the integration tests.
//!
//! Placed under `tests/common/mod.rs` (rather than a top-level
//! `tests/common.rs`) so cargo does not pick it up as a separate test
//! binary — each test file opts in with `mod common;`.

use std::collections::BTreeMap;

use frieze::{from_schemas, to_yaml, Info, OasVersion, Schemas};

/// Wraps a `Schemas` collection in a minimal `OasDocument` for snapshot
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
    let mut document = from_schemas(snapshot_info(), schemas, active_oas_version())
        .expect("from_schemas with active OAS version must succeed");
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

/// The OAS version the current build was compiled to emit. Used by
/// snapshot tests to satisfy the transition guard in `from_schemas`
/// without hard-coding a specific version per feature.
fn active_oas_version() -> OasVersion {
    #[cfg(feature = "oas-3-0")]
    {
        OasVersion::V3_0
    }
    #[cfg(feature = "oas-3-1")]
    {
        OasVersion::V3_1
    }
}
