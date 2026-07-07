//! The filesystem-backed [`MetadataSource`] implementation.

use frieze_model::{
    CargoFeatureName, OasVersionCheck, OutputConfig, OutputFilePath, OutputName, PackageMetadata,
    PackageName, PackageRoot, PartialFilePath,
};
use frieze_usecase::{Error, MetadataReadCause, MetadataSource, Result};
use toml::Value;

/// The keys the `[package.metadata.frieze]` table may carry today.
///
/// Anything else is rejected with a curated error instead of being
/// silently ignored — a typo like `outpts` must not turn into "no
/// outputs were generated".
const KNOWN_SECTION_KEYS: &[&str] = &["features", "oas-version", "outputs"];

/// The keys one `[[package.metadata.frieze.outputs]]` entry may carry.
const KNOWN_OUTPUT_KEYS: &[&str] = &["name", "partial", "output"];

const SECTION_TABLE: &str = "[package.metadata.frieze]";
const OUTPUTS_TABLE: &str = "[[package.metadata.frieze.outputs]]";

/// Reads a package's generation configuration from the
/// `[package.metadata.frieze]` section of its `Cargo.toml`.
///
/// Every raw value is converted to its parsed domain type on the way
/// in: output names become [`OutputName`], path strings are resolved
/// **relative to the package root** (the directory containing the
/// `Cargo.toml`) and validated as [`PartialFilePath`] /
/// [`OutputFilePath`], and each output's serialization format is
/// lifted from its output path's extension.
///
/// Unknown keys under the frieze tables are rejected with a curated
/// error, as are missing required keys and values of the wrong TOML
/// type.
#[derive(Debug, Default)]
pub struct FsMetadataSource;

impl FsMetadataSource {
    pub fn new() -> Self {
        Self
    }
}

impl MetadataSource for FsMetadataSource {
    fn read(&self, root: &PackageRoot) -> Result<PackageMetadata> {
        let manifest =
            read_manifest(root).map_err(|error| metadata_read(root, manifest_cause(error)))?;
        let package = manifest
            .get("package")
            .and_then(Value::as_table)
            .ok_or_else(|| metadata_read(root, MetadataReadCause::MissingPackageTable))?;
        let package_name = PackageName::new(required_str(package, "name", "[package]", root)?)?;
        let frieze = frieze_section(root, package)?;
        reject_unknown_keys(frieze, KNOWN_SECTION_KEYS, SECTION_TABLE, root)?;
        // The parent table's own values are validated before the
        // structural "at least one output" requirement, so a broken
        // declaration is reported as what it is.
        let features = features(root, frieze)?;
        let oas_version_check = oas_version_check(root, frieze)?;
        let outputs = outputs_entries(root, frieze)?
            .iter()
            .map(|entry| output_config(root, entry))
            .collect::<Result<Vec<_>>>()?;
        Ok(PackageMetadata::new(
            package_name,
            outputs,
            features,
            oas_version_check,
        )?)
    }
}

/// Parses the optional `features` array: the cargo features to enable
/// on the target crate while its schemas are collected, shared by
/// every output. Absent means none.
fn features(
    root: &PackageRoot,
    frieze: &toml::map::Map<String, Value>,
) -> Result<Vec<CargoFeatureName>> {
    let Some(value) = frieze.get("features") else {
        return Ok(Vec::new());
    };
    let entries = value.as_array().ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::UnexpectedType {
                key: "features".to_string(),
                table: SECTION_TABLE.to_string(),
                expected: "an array of strings",
            },
        )
    })?;
    entries
        .iter()
        .map(|entry| {
            let name = entry.as_str().ok_or_else(|| {
                metadata_read(
                    root,
                    MetadataReadCause::UnexpectedType {
                        key: "features".to_string(),
                        table: SECTION_TABLE.to_string(),
                        expected: "an array of strings",
                    },
                )
            })?;
            Ok(CargoFeatureName::new(name)?)
        })
        .collect()
}

/// Parses the optional `oas-version` string: a major.minor line
/// (`"3.0"` / `"3.1"`) every partial document must match. Absent means
/// the partials rule, unchecked.
fn oas_version_check(
    root: &PackageRoot,
    frieze: &toml::map::Map<String, Value>,
) -> Result<Option<OasVersionCheck>> {
    let Some(value) = frieze.get("oas-version") else {
        return Ok(None);
    };
    let raw = value.as_str().ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::UnexpectedType {
                key: "oas-version".to_string(),
                table: SECTION_TABLE.to_string(),
                expected: "a string",
            },
        )
    })?;
    Ok(Some(OasVersionCheck::new(raw)?))
}

/// Reads and parses the `Cargo.toml` under `root`. Raw failures stay
/// in the gateway's internal error here; the caller maps them into the
/// semantic boundary cause at the trait boundary.
fn read_manifest(root: &PackageRoot) -> std::result::Result<Value, crate::Error> {
    let raw = std::fs::read_to_string(root.cargo_toml())?;
    Ok(toml::from_str(&raw)?)
}

/// Maps the internal failures of [`read_manifest`] to their semantic
/// causes.
fn manifest_cause(error: crate::Error) -> MetadataReadCause {
    match error {
        crate::Error::Io(cause) => MetadataReadCause::CargoManifestRead(cause),
        crate::Error::Toml(cause) => MetadataReadCause::CargoManifestParse {
            message: cause.to_string(),
        },
        // The manifest path routes only through I/O and TOML.
        crate::Error::Yaml(_) | crate::Error::Json(_) => {
            unreachable!("Cargo.toml reading uses no YAML/JSON codec")
        }
    }
}

/// Locates the `[package.metadata.frieze]` table, distinguishing "the
/// section is absent" (a dedicated, actionable error) from "the
/// section exists but is not a table".
fn frieze_section<'a>(
    root: &PackageRoot,
    package: &'a toml::map::Map<String, Value>,
) -> Result<&'a toml::map::Map<String, Value>> {
    let section = package
        .get("metadata")
        .and_then(Value::as_table)
        .and_then(|metadata| metadata.get("frieze"))
        .ok_or_else(|| Error::MissingFriezeSection { root: root.clone() })?;
    section.as_table().ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::UnexpectedType {
                key: "frieze".to_string(),
                table: "[package.metadata]".to_string(),
                expected: "a table",
            },
        )
    })
}

/// Extracts the `outputs` array of tables, requiring at least one
/// entry.
fn outputs_entries<'a>(
    root: &PackageRoot,
    frieze: &'a toml::map::Map<String, Value>,
) -> Result<&'a [Value]> {
    let entries = match frieze.get("outputs") {
        None => &[] as &[Value],
        Some(value) => value.as_array().map(Vec::as_slice).ok_or_else(|| {
            metadata_read(
                root,
                MetadataReadCause::UnexpectedType {
                    key: "outputs".to_string(),
                    table: SECTION_TABLE.to_string(),
                    expected: "an array of tables",
                },
            )
        })?,
    };
    if entries.is_empty() {
        return Err(Error::NoOutputsDefined { root: root.clone() });
    }
    Ok(entries)
}

/// Parses one `[[package.metadata.frieze.outputs]]` entry into an
/// [`OutputConfig`], resolving its paths against the package root.
fn output_config(root: &PackageRoot, entry: &Value) -> Result<OutputConfig> {
    let table = entry.as_table().ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::UnexpectedType {
                key: "outputs".to_string(),
                table: SECTION_TABLE.to_string(),
                expected: "an array of tables",
            },
        )
    })?;
    reject_unknown_keys(table, KNOWN_OUTPUT_KEYS, OUTPUTS_TABLE, root)?;
    let name = OutputName::new(required_str(table, "name", OUTPUTS_TABLE, root)?)?;
    // `Path::join` keeps an absolute right-hand side as-is, so both
    // package-relative and absolute declarations resolve naturally.
    let partial = root
        .as_path()
        .join(required_str(table, "partial", OUTPUTS_TABLE, root)?);
    let output = root
        .as_path()
        .join(required_str(table, "output", OUTPUTS_TABLE, root)?);
    Ok(OutputConfig::new(
        name,
        PartialFilePath::try_from_path(partial)?,
        OutputFilePath::try_from_path(output)?,
    ))
}

/// Fetches a required string-valued key from `table`, with curated
/// errors for both absence and a wrong TOML value type.
fn required_str<'a>(
    table: &'a toml::map::Map<String, Value>,
    key: &str,
    table_name: &str,
    root: &PackageRoot,
) -> Result<&'a str> {
    let value = table.get(key).ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::MissingKey {
                key: key.to_string(),
                table: table_name.to_string(),
            },
        )
    })?;
    value.as_str().ok_or_else(|| {
        metadata_read(
            root,
            MetadataReadCause::UnexpectedType {
                key: key.to_string(),
                table: table_name.to_string(),
                expected: "a string",
            },
        )
    })
}

/// Rejects any key of `table` outside `known`, so typos surface as
/// errors instead of silently changing behaviour.
fn reject_unknown_keys(
    table: &toml::map::Map<String, Value>,
    known: &[&str],
    table_name: &str,
    root: &PackageRoot,
) -> Result<()> {
    match table.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(metadata_read(
            root,
            MetadataReadCause::UnknownKey {
                key: key.clone(),
                table: table_name.to_string(),
            },
        )),
        None => Ok(()),
    }
}

fn metadata_read(root: &PackageRoot, cause: MetadataReadCause) -> Error {
    Error::MetadataRead {
        root: root.clone(),
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frieze_model::OutputFormat;

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PackageRoot,
    }

    /// Writes a package directory containing a `Cargo.toml` with the
    /// given content plus the partial files the happy-path entries
    /// reference.
    fn fixture(manifest: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
        std::fs::create_dir_all(dir.path().join("openapi")).unwrap();
        for name in ["partial.yaml", "internal-partial.yaml"] {
            std::fs::write(dir.path().join("openapi").join(name), "openapi: 3.0.3\n").unwrap();
        }
        let root = PackageRoot::try_from_path(dir.path()).unwrap();
        Fixture { _dir: dir, root }
    }

    const HAPPY_MANIFEST: &str = r#"
[package]
name = "my-api"
version = "0.1.0"

[[package.metadata.frieze.outputs]]
name    = "default"
partial = "openapi/partial.yaml"
output  = "openapi/openapi.yaml"

[[package.metadata.frieze.outputs]]
name    = "internal"
partial = "openapi/internal-partial.yaml"
output  = "openapi/internal.json"
"#;

    #[test]
    fn reads_a_two_output_configuration() {
        let fixture = fixture(HAPPY_MANIFEST);
        let metadata = FsMetadataSource::new().read(&fixture.root).unwrap();
        assert_eq!(metadata.package_name().as_str(), "my-api");
        // Neither optional parent-table key is declared.
        assert!(metadata.features().is_empty());
        assert_eq!(metadata.oas_version_check(), None);
        assert_eq!(metadata.outputs().len(), 2);
        let first = &metadata.outputs()[0];
        assert_eq!(first.name().as_str(), "default");
        assert_eq!(
            first.partial().as_path(),
            fixture.root.as_path().join("openapi/partial.yaml")
        );
        assert_eq!(
            first.output().as_path(),
            fixture.root.as_path().join("openapi/openapi.yaml")
        );
        assert_eq!(first.format(), OutputFormat::Yaml);
        // The second output's format follows its own extension.
        assert_eq!(metadata.outputs()[1].format(), OutputFormat::Json);
    }

    #[test]
    fn reads_the_optional_parent_table_keys() {
        let manifest = r#"
[package]
name = "my-api"

[package.metadata.frieze]
features    = ["extra", "json-schema"]
oas-version = "3.1"

[[package.metadata.frieze.outputs]]
name    = "default"
partial = "openapi/partial.yaml"
output  = "openapi/openapi.yaml"
"#;
        let fixture = fixture(manifest);
        let metadata = FsMetadataSource::new().read(&fixture.root).unwrap();
        let features: Vec<&str> = metadata
            .features()
            .iter()
            .map(CargoFeatureName::as_str)
            .collect();
        assert_eq!(features, ["extra", "json-schema"]);
        assert_eq!(metadata.oas_version_check(), Some(OasVersionCheck::V3_1));
    }

    #[test]
    fn non_array_features_value_is_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\nfeatures = \"extra\"\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnexpectedType { key, expected, .. },
                    ..
                }) if key == "features" && *expected == "an array of strings"
            ),
            "expected the string features value to be rejected, got {result:?}"
        );
    }

    #[test]
    fn non_string_feature_entry_is_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\nfeatures = [1]\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnexpectedType { key, .. },
                    ..
                }) if key == "features"
            ),
            "expected the integer feature entry to be rejected, got {result:?}"
        );
    }

    #[test]
    fn invalid_feature_name_is_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\nfeatures = [\"dep:foo\"]\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::Config(
                    frieze_model::ConfigError::CargoFeatureNameInvalid { got }
                )) if got == "dep:foo"
            ),
            "expected the dependency-scoped feature to be rejected, got {result:?}"
        );
    }

    #[test]
    fn invalid_oas_version_value_lists_the_valid_literals() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\noas-version = \"3.0.3\"\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        match result {
            Err(Error::Config(error @ frieze_model::ConfigError::OasVersionCheckInvalid { .. })) => {
                let message = error.to_string();
                assert!(
                    message.contains("\"3.0\"") && message.contains("\"3.1\""),
                    "expected the valid literals to be listed, got: {message}"
                );
            }
            other => panic!("expected the patch-qualified value to be rejected, got {other:?}"),
        }
    }

    #[test]
    fn non_string_oas_version_value_is_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\n\"oas-version\" = 3.0\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnexpectedType { key, expected, .. },
                    ..
                }) if key == "oas-version" && *expected == "a string"
            ),
            "expected the float value to be rejected, got {result:?}"
        );
    }

    #[test]
    fn missing_frieze_section_is_a_dedicated_error() {
        let fixture = fixture("[package]\nname = \"my-api\"\n");
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(result, Err(Error::MissingFriezeSection { .. })),
            "expected the missing section to be rejected, got {result:?}"
        );
    }

    #[test]
    fn empty_outputs_are_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\noutputs = []\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(result, Err(Error::NoOutputsDefined { .. })),
            "expected the empty outputs array to be rejected, got {result:?}"
        );
    }

    #[test]
    fn unknown_section_key_is_rejected() {
        let manifest = "[package]\nname = \"my-api\"\n\
                        [package.metadata.frieze]\noutpts = []\n";
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnknownKey { key, table },
                    ..
                }) if key == "outpts" && table == SECTION_TABLE
            ),
            "expected the unknown key to be rejected, got {result:?}"
        );
    }

    #[test]
    fn unknown_output_entry_key_is_rejected() {
        let manifest = r#"
[package]
name = "my-api"

[[package.metadata.frieze.outputs]]
name    = "default"
parital = "openapi/partial.yaml"
output  = "openapi/openapi.yaml"
"#;
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnknownKey { key, table },
                    ..
                }) if key == "parital" && table == OUTPUTS_TABLE
            ),
            "expected the typo key to be rejected, got {result:?}"
        );
    }

    #[test]
    fn missing_required_output_key_is_rejected() {
        let manifest = r#"
[package]
name = "my-api"

[[package.metadata.frieze.outputs]]
name    = "default"
partial = "openapi/partial.yaml"
"#;
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::MissingKey { key, .. },
                    ..
                }) if key == "output"
            ),
            "expected the missing key to be rejected, got {result:?}"
        );
    }

    #[test]
    fn non_string_value_is_rejected() {
        let manifest = r#"
[package]
name = "my-api"

[[package.metadata.frieze.outputs]]
name    = 1
partial = "openapi/partial.yaml"
output  = "openapi/openapi.yaml"
"#;
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                &result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::UnexpectedType { key, expected, .. },
                    ..
                }) if key == "name" && *expected == "a string"
            ),
            "expected the integer name to be rejected, got {result:?}"
        );
    }

    #[test]
    fn invalid_toml_is_a_parse_error() {
        let fixture = fixture("[package\nname = \"broken\"\n");
        let result = FsMetadataSource::new().read(&fixture.root);
        assert!(
            matches!(
                result,
                Err(Error::MetadataRead {
                    cause: MetadataReadCause::CargoManifestParse { .. },
                    ..
                })
            ),
            "expected the broken manifest to be a parse error, got {result:?}"
        );
    }

    #[test]
    fn missing_partial_file_reports_the_resolved_path() {
        let manifest = r#"
[package]
name = "my-api"

[[package.metadata.frieze.outputs]]
name    = "default"
partial = "openapi/no-such.yaml"
output  = "openapi/openapi.yaml"
"#;
        let fixture = fixture(manifest);
        let result = FsMetadataSource::new().read(&fixture.root);
        match result {
            Err(Error::Config(frieze_model::ConfigError::PartialFileNotFound { got })) => {
                assert!(got.is_absolute(), "expected a resolved absolute path");
                assert!(got.ends_with("openapi/no-such.yaml"));
            }
            other => panic!("expected the missing partial to be rejected, got {other:?}"),
        }
    }
}
