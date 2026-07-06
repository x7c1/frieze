//! The parsed generation configuration of one package.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::cargo_feature_name::CargoFeatureName;
use crate::config_error::ConfigError;
use crate::oas_version_check::OasVersionCheck;
use crate::output_file_path::OutputFilePath;
use crate::output_format::OutputFormat;
use crate::output_name::OutputName;
use crate::package_name::PackageName;
use crate::partial_file_path::PartialFilePath;

/// One declared output of a package: which partial document to start
/// from, where to write the result, and in which format.
///
/// Constructed via [`OutputConfig::new`]; the fields are private so
/// the construction path stays controlled. The constructor is
/// infallible because every part arrives already validated and the
/// format is lifted from the output path's extension — there is no
/// cross-field mismatch left to reject.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    name: OutputName,
    partial: PartialFilePath,
    output: OutputFilePath,
    format: OutputFormat,
}

impl OutputConfig {
    /// Assembles an output configuration. The serialization format is
    /// the one lifted from `output`'s file extension.
    pub fn new(name: OutputName, partial: PartialFilePath, output: OutputFilePath) -> Self {
        let format = output.format();
        Self {
            name,
            partial,
            output,
            format,
        }
    }

    pub fn name(&self) -> &OutputName {
        &self.name
    }

    pub fn partial(&self) -> &PartialFilePath {
        &self.partial
    }

    pub fn output(&self) -> &OutputFilePath {
        &self.output
    }

    /// The serialization format of this output (lifted from the output
    /// path's extension).
    pub fn format(&self) -> OutputFormat {
        self.format
    }
}

/// The parsed generation configuration of one package: its name, its
/// declared outputs, the cargo features to enable while collecting
/// schemas, and an optional OAS-version consistency check.
///
/// Constructed via [`PackageMetadata::new`]; the fields are private so
/// the invariants cannot be bypassed.
///
/// # Invariants
///
/// - every output name is unique within the package
/// - every output path is unique within the package (compared on the
///   configured path value); two outputs writing to the same file
///   would silently overwrite each other
///
/// Whether at least one output must be declared is a policy of the
/// configuration *source* (readers reject a package that declares
/// none), not of this aggregate — an empty list is representable.
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    package_name: PackageName,
    outputs: Vec<OutputConfig>,
    features: Vec<CargoFeatureName>,
    oas_version_check: Option<OasVersionCheck>,
}

impl PackageMetadata {
    /// Assembles the package configuration, rejecting duplicate output
    /// names ([`ConfigError::OutputNameCollision`]) and duplicate
    /// output paths ([`ConfigError::OutputPathCollision`]).
    pub fn new(
        package_name: PackageName,
        outputs: Vec<OutputConfig>,
        features: Vec<CargoFeatureName>,
        oas_version_check: Option<OasVersionCheck>,
    ) -> Result<Self, ConfigError> {
        let mut seen_names = BTreeSet::new();
        for config in &outputs {
            if !seen_names.insert(config.name().clone()) {
                return Err(ConfigError::OutputNameCollision {
                    name: config.name().clone(),
                });
            }
        }
        let mut users: BTreeMap<PathBuf, Vec<OutputName>> = BTreeMap::new();
        for config in &outputs {
            users
                .entry(config.output().as_path().to_path_buf())
                .or_default()
                .push(config.name().clone());
        }
        if let Some((path, used_by)) = users.into_iter().find(|(_, names)| names.len() > 1) {
            return Err(ConfigError::OutputPathCollision { path, used_by });
        }
        Ok(Self {
            package_name,
            outputs,
            features,
            oas_version_check,
        })
    }

    pub fn package_name(&self) -> &PackageName {
        &self.package_name
    }

    pub fn outputs(&self) -> &[OutputConfig] {
        &self.outputs
    }

    pub fn features(&self) -> &[CargoFeatureName] {
        &self.features
    }

    /// The optional OAS major.minor version the metadata pins as a
    /// consistency check against each partial document's declared
    /// version.
    pub fn oas_version_check(&self) -> Option<OasVersionCheck> {
        self.oas_version_check
    }

    /// Looks up the output declared under `name`, if any.
    pub fn find_by_name(&self, name: &OutputName) -> Option<&OutputConfig> {
        self.outputs.iter().find(|config| config.name() == name)
    }

    /// The declared output names, in declaration order.
    pub fn output_names(&self) -> Vec<OutputName> {
        self.outputs
            .iter()
            .map(|config| config.name().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_config(dir: &std::path::Path, name: &str, file_stem: &str) -> OutputConfig {
        let partial_path = dir.join(format!("{file_stem}-partial.yaml"));
        std::fs::write(&partial_path, "openapi: 3.0.3\n").unwrap();
        OutputConfig::new(
            OutputName::new(name).unwrap(),
            PartialFilePath::try_from_path(&partial_path).unwrap(),
            OutputFilePath::try_from_path(dir.join(format!("{file_stem}.yaml"))).unwrap(),
        )
    }

    fn package_name() -> PackageName {
        PackageName::new("my-api").unwrap()
    }

    #[test]
    fn accepts_outputs_with_unique_names_and_paths() {
        let dir = tempfile::tempdir().unwrap();
        let outputs = vec![
            output_config(dir.path(), "public", "public"),
            output_config(dir.path(), "internal", "internal"),
        ];
        let metadata = PackageMetadata::new(
            package_name(),
            outputs,
            vec![CargoFeatureName::new("extra").unwrap()],
            Some(OasVersionCheck::V3_0),
        )
        .unwrap();
        assert_eq!(metadata.outputs().len(), 2);
        assert_eq!(
            metadata.output_names(),
            vec![
                OutputName::new("public").unwrap(),
                OutputName::new("internal").unwrap()
            ]
        );
        assert_eq!(metadata.oas_version_check(), Some(OasVersionCheck::V3_0));
        let found = metadata.find_by_name(&OutputName::new("internal").unwrap());
        assert!(found.is_some());
        assert!(metadata
            .find_by_name(&OutputName::new("absent").unwrap())
            .is_none());
    }

    #[test]
    fn rejects_a_duplicate_output_name() {
        let dir = tempfile::tempdir().unwrap();
        let outputs = vec![
            output_config(dir.path(), "public", "a"),
            output_config(dir.path(), "public", "b"),
        ];
        let result = PackageMetadata::new(package_name(), outputs, Vec::new(), None);
        assert!(
            matches!(
                &result,
                Err(ConfigError::OutputNameCollision { name }) if name.as_str() == "public"
            ),
            "expected the duplicate name to be rejected, got {result:?}"
        );
    }

    #[test]
    fn rejects_a_duplicate_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let shared_output = OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap();
        let partial_path = dir.path().join("partial.yaml");
        std::fs::write(&partial_path, "openapi: 3.0.3\n").unwrap();
        let partial = PartialFilePath::try_from_path(&partial_path).unwrap();
        let outputs = vec![
            OutputConfig::new(
                OutputName::new("public").unwrap(),
                partial.clone(),
                shared_output.clone(),
            ),
            OutputConfig::new(OutputName::new("internal").unwrap(), partial, shared_output),
        ];
        let result = PackageMetadata::new(package_name(), outputs, Vec::new(), None);
        assert!(
            matches!(
                &result,
                Err(ConfigError::OutputPathCollision { used_by, .. }) if used_by.len() == 2
            ),
            "expected the duplicate path to be rejected, got {result:?}"
        );
    }

    #[test]
    fn output_config_lifts_the_format_from_the_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let partial_path = dir.path().join("partial.yaml");
        std::fs::write(&partial_path, "openapi: 3.0.3\n").unwrap();
        let config = OutputConfig::new(
            OutputName::new("public").unwrap(),
            PartialFilePath::try_from_path(&partial_path).unwrap(),
            OutputFilePath::try_from_path(dir.path().join("openapi.json")).unwrap(),
        );
        assert_eq!(config.format(), OutputFormat::Json);
        assert_eq!(config.partial().format(), OutputFormat::Yaml);
    }
}
