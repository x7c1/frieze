//! The `GenerateOas` interactor: the end-to-end generate flow,
//! expressed against the gateway traits.

use frieze_model::{OutputConfig, OutputFilePath, OutputName, PackageMetadata, PackageRoot};

use crate::compose::compose_components;
use crate::gateway::{MetadataSource, OutputSink, PartialSource, SchemasCollector};
use crate::{Error, Result};

/// The input of one [`GenerateOas::run`] invocation.
pub struct GenerateOasParams {
    /// The package whose configuration and schemas drive the run.
    pub root: PackageRoot,
    /// When set, only the output declared under this name is
    /// generated; when `None`, every declared output is.
    pub filter: Option<OutputName>,
}

/// One output a [`GenerateOas::run`] invocation wrote: its declared
/// name and the path the document landed at.
#[derive(Debug)]
pub struct WrittenOutput {
    /// The declared name of the output.
    pub name: OutputName,
    /// The path the generated document was written to.
    pub path: OutputFilePath,
}

/// The result of a successful [`GenerateOas::run`].
#[derive(Debug)]
pub struct Report {
    /// The outputs that were generated, in the order they were
    /// written.
    pub written: Vec<WrittenOutput>,
}

impl Report {
    /// Wraps the list of generated outputs.
    pub fn success(written: Vec<WrittenOutput>) -> Self {
        Self { written }
    }
}

/// Orchestrates the generate flow: read the package configuration,
/// collect the schemas the target crate registers, and for each
/// selected output compose the schemas into its partial document and
/// persist the result.
///
/// The interactor holds one implementation per gateway trait and is
/// generic over the concrete types; the composition root decides which
/// implementations are injected. Schemas are collected once per run
/// and reused across every output — only the partial document and the
/// serialization format vary per output.
pub struct GenerateOas<M, S, P, O> {
    metadata: M,
    schemas: S,
    partials: P,
    outputs: O,
}

impl<M, S, P, O> GenerateOas<M, S, P, O>
where
    M: MetadataSource,
    S: SchemasCollector,
    P: PartialSource,
    O: OutputSink,
{
    /// Assembles the interactor from its four gateways.
    pub fn new(metadata: M, schemas: S, partials: P, outputs: O) -> Self {
        Self {
            metadata,
            schemas,
            partials,
            outputs,
        }
    }

    /// Runs the generate flow for `params.root`, returning the names
    /// of the outputs that were written.
    ///
    /// Outputs are processed in declaration order and the flow stops
    /// at the first failure; a filter naming an undeclared output
    /// fails with [`Error::UnknownOutputName`] before anything is
    /// read or written.
    pub fn run(&self, params: &GenerateOasParams) -> Result<Report> {
        let metadata = self.metadata.read(&params.root)?;
        let selected = select_outputs(&metadata, params.filter.as_ref())?;
        let components = self.schemas.collect(&params.root, &metadata)?;
        let mut written = Vec::new();
        for config in selected {
            let partial = self.partials.load(config.partial())?;
            let complete = compose_components(partial, components.clone())?;
            self.outputs
                .persist(config.output(), &complete, config.format())?;
            written.push(WrittenOutput {
                name: config.name().clone(),
                path: config.output().clone(),
            });
        }
        Ok(Report::success(written))
    }
}

/// Resolves the outputs a run should generate: all of them, or the
/// single one named by `filter`.
fn select_outputs<'a>(
    metadata: &'a PackageMetadata,
    filter: Option<&OutputName>,
) -> Result<Vec<&'a OutputConfig>> {
    match filter {
        Some(name) => metadata
            .find_by_name(name)
            .map(|config| vec![config])
            .ok_or_else(|| Error::UnknownOutputName {
                requested: name.clone(),
                available: metadata.output_names(),
            }),
        None => Ok(metadata.outputs().iter().collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::path::PathBuf;

    use frieze_model::{
        OutputConfig, OutputFilePath, OutputFormat, PackageMetadata, PackageName, PartialFilePath,
    };
    use frieze_openapi::{Components, Document, Info, Version};

    /// An in-memory fixture: a package with two declared outputs
    /// (`public` → YAML, `internal` → JSON) backed by real temp files
    /// so the parsed path types can be constructed.
    struct Fixture {
        _dir: tempfile::TempDir,
        metadata: PackageMetadata,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let mut outputs = Vec::new();
        for (name, output_file) in [("public", "public.yaml"), ("internal", "internal.json")] {
            let partial_path = dir.path().join(format!("{name}-partial.yaml"));
            std::fs::write(&partial_path, "openapi: 3.0.3\n").unwrap();
            outputs.push(OutputConfig::new(
                OutputName::new(name).unwrap(),
                PartialFilePath::try_from_path(&partial_path).unwrap(),
                OutputFilePath::try_from_path(dir.path().join(output_file)).unwrap(),
            ));
        }
        let metadata = PackageMetadata::new(
            PackageName::new("my-api").unwrap(),
            outputs,
            Vec::new(),
            None,
        )
        .unwrap();
        Fixture {
            _dir: dir,
            metadata,
        }
    }

    struct FakeMetadataSource {
        metadata: PackageMetadata,
    }

    impl MetadataSource for FakeMetadataSource {
        fn read(&self, _root: &PackageRoot) -> Result<PackageMetadata> {
            Ok(self.metadata.clone())
        }
    }

    struct FakeSchemasCollector {
        components: Components,
        calls: RefCell<usize>,
    }

    impl SchemasCollector for FakeSchemasCollector {
        fn collect(&self, _root: &PackageRoot, _metadata: &PackageMetadata) -> Result<Components> {
            *self.calls.borrow_mut() += 1;
            Ok(self.components.clone())
        }
    }

    struct FakePartialSource;

    impl PartialSource for FakePartialSource {
        fn load(&self, _path: &PartialFilePath) -> Result<Document> {
            Ok(Document::from_components(
                Info::default(),
                Components::default(),
                Version::V3_0,
            ))
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        persisted: RefCell<Vec<(PathBuf, OutputFormat, usize)>>,
    }

    impl OutputSink for RecordingSink {
        fn persist(
            &self,
            target: &OutputFilePath,
            document: &Document,
            format: OutputFormat,
        ) -> Result<()> {
            let schema_count = document
                .components
                .as_ref()
                .map_or(0, |components| components.schemas.len());
            self.persisted.borrow_mut().push((
                target.as_path().to_path_buf(),
                format,
                schema_count,
            ));
            Ok(())
        }
    }

    fn one_schema_components() -> Components {
        serde_json::from_value(serde_json::json!({
            "schemas": { "User": { "type": "object" } }
        }))
        .unwrap()
    }

    fn interactor(
        fixture: &Fixture,
    ) -> GenerateOas<FakeMetadataSource, FakeSchemasCollector, FakePartialSource, RecordingSink>
    {
        GenerateOas::new(
            FakeMetadataSource {
                metadata: fixture.metadata.clone(),
            },
            FakeSchemasCollector {
                components: one_schema_components(),
                calls: RefCell::new(0),
            },
            FakePartialSource,
            RecordingSink::default(),
        )
    }

    fn package_root(fixture: &Fixture) -> PackageRoot {
        std::fs::write(fixture._dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        PackageRoot::try_from_path(fixture._dir.path()).unwrap()
    }

    #[test]
    fn generates_every_declared_output_on_the_happy_path() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            root: package_root(&fixture),
            filter: None,
        };

        let report = interactor.run(&params).unwrap();

        let names: Vec<&str> = report
            .written
            .iter()
            .map(|written| written.name.as_str())
            .collect();
        assert_eq!(names, ["public", "internal"]);
        // Each written entry reports the path the output landed at.
        assert!(report.written[0].path.as_path().ends_with("public.yaml"));
        assert!(report.written[1].path.as_path().ends_with("internal.json"));
        // Schemas are collected once and reused across outputs.
        assert_eq!(*interactor.schemas.calls.borrow(), 1);
        let persisted = interactor.outputs.persisted.borrow();
        assert_eq!(persisted.len(), 2);
        assert!(persisted[0].0.ends_with("public.yaml"));
        assert_eq!(persisted[0].1, OutputFormat::Yaml);
        assert!(persisted[1].0.ends_with("internal.json"));
        assert_eq!(persisted[1].1, OutputFormat::Json);
        // Each persisted document received the collected schema.
        assert!(persisted.iter().all(|(_, _, count)| *count == 1));
    }

    #[test]
    fn filter_selects_a_single_output() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            root: package_root(&fixture),
            filter: Some(OutputName::new("internal").unwrap()),
        };

        let report = interactor.run(&params).unwrap();

        let names: Vec<&str> = report
            .written
            .iter()
            .map(|written| written.name.as_str())
            .collect();
        assert_eq!(names, ["internal"]);
        let persisted = interactor.outputs.persisted.borrow();
        assert_eq!(persisted.len(), 1);
        assert!(persisted[0].0.ends_with("internal.json"));
    }

    #[test]
    fn unknown_filter_fails_before_writing_anything() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            root: package_root(&fixture),
            filter: Some(OutputName::new("absent").unwrap()),
        };

        let result = interactor.run(&params);

        assert!(
            matches!(
                &result,
                Err(Error::UnknownOutputName { requested, available })
                    if requested.as_str() == "absent" && available.len() == 2
            ),
            "expected the unknown output name to be rejected, got {:?}",
            result.map(|report| report.written)
        );
        assert!(interactor.outputs.persisted.borrow().is_empty());
        // The filter is resolved before schemas are collected.
        assert_eq!(*interactor.schemas.calls.borrow(), 0);
    }
}
