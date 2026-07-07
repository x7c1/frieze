//! Package inspection via `cargo metadata`.
//!
//! Everything the collector needs to know about the target package
//! before it can generate and run the scratch crate: where the build
//! directory lives, where the workspace root (and thus its
//! `Cargo.lock`) is, which lib crate the scratch binary must import,
//! and how the package declares its `frieze` dependency.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use frieze_model::{PackageName, PackageRoot};
use serde_json::Value;

use crate::Error;

/// What `cargo metadata` reports about the target package.
#[derive(Debug)]
pub(crate) struct PackageInspection {
    /// The build directory (`target_directory`), honoring
    /// `CARGO_TARGET_DIR` and `[build] target-dir`; the scratch crate
    /// lives under `<target_directory>/frieze/<package>/`.
    pub target_directory: PathBuf,
    /// The workspace root; its `Cargo.lock` seeds the scratch crate's
    /// lockfile.
    pub workspace_root: PathBuf,
    /// The crate name of the package's lib target (dashes already
    /// replaced by underscores) — what the scratch `main.rs` imports.
    pub lib_crate_name: String,
    /// The package's declared dependency on `frieze`, when it has one.
    pub frieze_dependency: Option<FriezeDependency>,
}

/// The shape of the target package's declared `frieze` dependency.
#[derive(Debug)]
pub(crate) struct FriezeDependency {
    pub(crate) uses_default_features: bool,
    pub(crate) features: Vec<String>,
    /// The declared version requirement, as cargo normalizes it
    /// (`"0.1"` becomes `"^0.1"`; a requirement-less path dependency
    /// reports `"*"`).
    pub(crate) requirement: String,
    /// The resolved directory of a path dependency; `None` for
    /// registry and git sources.
    pub(crate) path: Option<PathBuf>,
    /// The dependency source (`registry+...`, `git+...`); `None` for
    /// path dependencies.
    pub(crate) source: Option<String>,
}

impl FriezeDependency {
    /// Whether the declaration leaves the `inventory` feature on.
    ///
    /// `inventory` is a default feature, so it is enabled unless the
    /// user opts out with `default-features = false` — and even then a
    /// re-listed `"inventory"` (or `"default"`) in `features` turns it
    /// back on.
    pub fn inventory_enabled(&self) -> bool {
        self.uses_default_features
            || self
                .features
                .iter()
                .any(|feature| feature == "inventory" || feature == "default")
    }
}

/// The cargo binary to invoke: whatever cargo says via `$CARGO` when
/// this process runs under a cargo subcommand, or plain `cargo`.
pub(crate) fn cargo_bin() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

/// Runs `cargo metadata` for the package at `root` and extracts the
/// inspection. A failed invocation carries cargo's captured stderr
/// verbatim; the collector does not reformat it.
pub(crate) fn inspect_package(
    root: &PackageRoot,
    package_name: &PackageName,
) -> Result<PackageInspection, Error> {
    let output = Command::new(cargo_bin())
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(root.cargo_toml())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|cause| Error::CargoInvocation {
            exit_code: None,
            stderr: cause.to_string(),
        })?;
    if !output.status.success() {
        return Err(Error::CargoInvocation {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let json: Value = serde_json::from_slice(&output.stdout).map_err(|cause| {
        inspect_error(format!(
            "cargo metadata did not produce valid JSON: {cause}"
        ))
    })?;
    parse_inspection(&json, package_name)
}

/// Extracts the [`PackageInspection`] from a parsed `cargo metadata`
/// document. Split from the subprocess call so the extraction logic is
/// testable against canned documents.
pub(crate) fn parse_inspection(
    json: &Value,
    package_name: &PackageName,
) -> Result<PackageInspection, Error> {
    let target_directory = required_path(json, "target_directory")?;
    let workspace_root = required_path(json, "workspace_root")?;
    let package = json["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|package| package["name"].as_str() == Some(package_name.as_str()))
        .ok_or_else(|| {
            inspect_error(format!(
                "package `{package_name}` does not appear in the cargo metadata output"
            ))
        })?;
    let lib_target_name = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|target| {
            target["kind"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| matches!(kind.as_str(), Some("lib") | Some("rlib")))
        })
        .and_then(|target| target["name"].as_str())
        .ok_or_else(|| {
            inspect_error(format!(
                "package `{package_name}` has no lib target; \
                 the scratch binary links the package as a library to collect its schemas"
            ))
        })?;
    let frieze_dependency = package["dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        // Only a normal dependency (`kind: null`) makes the derive
        // output part of the lib target; a dev- or build-dependency
        // registers nothing the scratch binary could collect.
        .find(|dependency| {
            dependency["name"].as_str() == Some("frieze") && dependency["kind"].is_null()
        })
        .map(|dependency| FriezeDependency {
            uses_default_features: dependency["uses_default_features"]
                .as_bool()
                .unwrap_or(true),
            features: dependency["features"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|feature| feature.as_str().map(str::to_owned))
                .collect(),
            requirement: dependency["req"].as_str().unwrap_or("*").to_owned(),
            path: dependency["path"].as_str().map(PathBuf::from),
            source: dependency["source"].as_str().map(str::to_owned),
        });
    Ok(PackageInspection {
        target_directory,
        workspace_root,
        lib_crate_name: lib_target_name.replace('-', "_"),
        frieze_dependency,
    })
}

fn required_path(json: &Value, key: &str) -> Result<PathBuf, Error> {
    json[key]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| inspect_error(format!("cargo metadata output has no `{key}` field")))
}

fn inspect_error(message: String) -> Error {
    Error::PackageInspect { message }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn canned_metadata(dependencies: Value) -> Value {
        serde_json::json!({
            "target_directory": "/work/my-api/target",
            "workspace_root": "/work/my-api",
            "packages": [{
                "name": "my-api",
                "targets": [
                    { "name": "some-bin", "kind": ["bin"] },
                    { "name": "my-api", "kind": ["lib"] }
                ],
                "dependencies": dependencies,
            }]
        })
    }

    fn package_name() -> PackageName {
        PackageName::new("my-api").unwrap()
    }

    #[test]
    fn extracts_directories_and_the_lib_crate_name() {
        let json = canned_metadata(serde_json::json!([]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        assert_eq!(
            inspection.target_directory,
            PathBuf::from("/work/my-api/target")
        );
        assert_eq!(inspection.workspace_root, PathBuf::from("/work/my-api"));
        // Dashes in the lib target name become underscores: this is
        // the name `use ... as _;` must spell.
        assert_eq!(inspection.lib_crate_name, "my_api");
        assert!(inspection.frieze_dependency.is_none());
    }

    #[test]
    fn default_frieze_dependency_has_inventory_enabled() {
        let json = canned_metadata(serde_json::json!([
            { "name": "frieze", "uses_default_features": true, "features": [] }
        ]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        assert!(inspection.frieze_dependency.unwrap().inventory_enabled());
    }

    #[test]
    fn opting_out_of_default_features_disables_inventory() {
        let json = canned_metadata(serde_json::json!([
            { "name": "frieze", "uses_default_features": false, "features": [] }
        ]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        assert!(!inspection.frieze_dependency.unwrap().inventory_enabled());
    }

    #[test]
    fn relisting_inventory_reenables_it() {
        for feature in ["inventory", "default"] {
            let json = canned_metadata(serde_json::json!([
                { "name": "frieze", "uses_default_features": false, "features": [feature] }
            ]));
            let inspection = parse_inspection(&json, &package_name()).unwrap();
            assert!(
                inspection.frieze_dependency.unwrap().inventory_enabled(),
                "feature `{feature}` should re-enable inventory"
            );
        }
    }

    #[test]
    fn the_dependency_requirement_path_and_source_are_extracted() {
        let json = canned_metadata(serde_json::json!([{
            "name": "frieze",
            "uses_default_features": true,
            "features": [],
            "req": "^0.1",
            "source": "registry+https://github.com/rust-lang/crates.io-index",
        }]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        let dependency = inspection.frieze_dependency.unwrap();
        assert_eq!(dependency.requirement, "^0.1");
        assert!(dependency.path.is_none());
        assert_eq!(
            dependency.source.as_deref(),
            Some("registry+https://github.com/rust-lang/crates.io-index")
        );

        let json = canned_metadata(serde_json::json!([{
            "name": "frieze",
            "uses_default_features": true,
            "features": [],
            "req": "*",
            "path": "/work/frieze/crates/libs/frieze",
            "source": null,
        }]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        let dependency = inspection.frieze_dependency.unwrap();
        assert_eq!(dependency.requirement, "*");
        assert_eq!(
            dependency.path.as_deref(),
            Some(Path::new("/work/frieze/crates/libs/frieze"))
        );
        assert!(dependency.source.is_none());
    }

    #[test]
    fn a_dev_only_frieze_dependency_does_not_count() {
        // Only a normal dependency links the derive output into the
        // lib target; a dev-dependency must not satisfy the check.
        let json = canned_metadata(serde_json::json!([{
            "name": "frieze",
            "uses_default_features": true,
            "features": [],
            "kind": "dev",
        }]));
        let inspection = parse_inspection(&json, &package_name()).unwrap();
        assert!(inspection.frieze_dependency.is_none());
    }

    #[test]
    fn a_package_without_a_lib_target_is_rejected() {
        let json = serde_json::json!({
            "target_directory": "/t",
            "workspace_root": "/w",
            "packages": [{
                "name": "my-api",
                "targets": [{ "name": "my-api", "kind": ["bin"] }],
                "dependencies": [],
            }]
        });
        let result = parse_inspection(&json, &package_name());
        assert!(
            matches!(
                &result,
                Err(Error::PackageInspect { message }) if message.contains("no lib target")
            ),
            "expected the bin-only package to be rejected, got {result:?}"
        );
    }

    #[test]
    fn a_missing_package_is_rejected() {
        let json = serde_json::json!({
            "target_directory": "/t",
            "workspace_root": "/w",
            "packages": []
        });
        let result = parse_inspection(&json, &package_name());
        assert!(
            matches!(result, Err(Error::PackageInspect { .. })),
            "expected the missing package to be rejected, got {result:?}"
        );
    }
}
