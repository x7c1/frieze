//! The `GenerateOas` interactor: the end-to-end generate flow,
//! expressed against the gateway traits.

use frieze_model::{
    OasVersionCheck, OutputConfig, OutputFilePath, OutputName, PackageMetadata, PackageName,
};
use frieze_openapi::{Document, Version};

use crate::compose::compose_components;
use crate::gateway::{
    MetadataSource, OutputSink, PackageResolver, PartialSource, SchemasCollector,
};
use crate::{Error, Result};

/// The input of one [`GenerateOas::run`] invocation.
pub struct GenerateOasParams {
    /// The explicitly requested target package, when the caller named
    /// one; when `None`, the resolver derives the target from the
    /// invocation's environment (workspace default, current
    /// directory).
    pub package: Option<PackageName>,
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

/// Orchestrates the generate flow: resolve the target package, read
/// its configuration, collect the schemas the target crate registers,
/// and for each selected output compose the schemas into its partial
/// document and persist the result.
///
/// The interactor holds one implementation per gateway trait and is
/// generic over the concrete types; the composition root decides which
/// implementations are injected. Schemas are collected once per run
/// and reused across every output — only the partial document and the
/// serialization format vary per output.
pub struct GenerateOas<R, M, S, P, O> {
    resolver: R,
    metadata: M,
    schemas: S,
    partials: P,
    outputs: O,
}

impl<R, M, S, P, O> GenerateOas<R, M, S, P, O>
where
    R: PackageResolver,
    M: MetadataSource,
    S: SchemasCollector,
    P: PartialSource,
    O: OutputSink,
{
    /// Assembles the interactor from its five gateways.
    pub fn new(resolver: R, metadata: M, schemas: S, partials: P, outputs: O) -> Self {
        Self {
            resolver,
            metadata,
            schemas,
            partials,
            outputs,
        }
    }

    /// Runs the generate flow for the resolved target package,
    /// returning the names of the outputs that were written.
    ///
    /// The target package is resolved first — every path in its
    /// configuration is then read relative to *that* package's root
    /// directory, never the workspace root.
    ///
    /// Outputs are processed in declaration order and the flow stops
    /// at the first failure; a filter naming an undeclared output
    /// fails with [`Error::UnknownOutputName`] before anything is
    /// read or written.
    ///
    /// Every selected partial is loaded — and checked against the
    /// metadata's optional `oas-version` declaration — *before* the
    /// schema collection: an unreadable or inconsistent partial fails
    /// the run without paying for a build, and before any output file
    /// is touched.
    pub fn run(&self, params: &GenerateOasParams) -> Result<Report> {
        let root = self.resolver.resolve(params.package.as_ref())?;
        let metadata = self.metadata.read(&root)?;
        let selected = select_outputs(&metadata, params.filter.as_ref())?;
        let partials = selected
            .iter()
            .map(|config| {
                let partial = self.partials.load(config.partial())?;
                check_oas_version(&metadata, config, &partial)?;
                Ok(partial)
            })
            .collect::<Result<Vec<_>>>()?;
        let components = self.schemas.collect(&root, &metadata)?;
        let mut written = Vec::new();
        for (config, partial) in selected.iter().zip(partials) {
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

/// Enforces the metadata's optional `oas-version` consistency check
/// against the version a partial document declares.
///
/// The generated document always follows the partial's `openapi:`
/// field; when the metadata pins a major.minor line, a partial outside
/// that line is rejected with [`Error::OasVersionMismatch`]. Without
/// the declaration the partials rule, unchecked.
fn check_oas_version(
    metadata: &PackageMetadata,
    config: &OutputConfig,
    partial: &Document,
) -> Result<()> {
    let Some(expected) = metadata.oas_version_check() else {
        return Ok(());
    };
    let matches = match expected {
        OasVersionCheck::V3_0 => partial.oas_version == Version::V3_0,
        OasVersionCheck::V3_1 => partial.oas_version == Version::V3_1,
    };
    if matches {
        return Ok(());
    }
    Err(Error::OasVersionMismatch {
        output: config.name().clone(),
        partial: config.partial().clone(),
        partial_version: partial.openapi.clone(),
        expected,
    })
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
        OutputConfig, OutputFilePath, OutputFormat, PackageMetadata, PackageRoot, PartialFilePath,
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
        fixture_with(None)
    }

    fn fixture_with(oas_version_check: Option<OasVersionCheck>) -> Fixture {
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
            oas_version_check,
        )
        .unwrap();
        Fixture {
            _dir: dir,
            metadata,
        }
    }

    /// Resolves every request to the fixture's root, recording the
    /// explicitly requested package name it received.
    struct FakeResolver {
        root: PackageRoot,
        requested: RefCell<Vec<Option<PackageName>>>,
    }

    impl PackageResolver for FakeResolver {
        fn resolve(&self, package: Option<&PackageName>) -> Result<PackageRoot> {
            self.requested.borrow_mut().push(package.cloned());
            Ok(self.root.clone())
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
    ) -> GenerateOas<
        FakeResolver,
        FakeMetadataSource,
        FakeSchemasCollector,
        FakePartialSource,
        RecordingSink,
    > {
        GenerateOas::new(
            FakeResolver {
                root: package_root(fixture),
                requested: RefCell::new(Vec::new()),
            },
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
            package: None,
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
    fn the_requested_package_reaches_the_resolver() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            package: Some(PackageName::new("my-api").unwrap()),
            filter: None,
        };

        interactor.run(&params).unwrap();

        // Resolution happens exactly once, with the explicit request
        // forwarded verbatim.
        let requested = interactor.resolver.requested.borrow();
        assert_eq!(requested.len(), 1);
        assert_eq!(
            requested[0].as_ref().map(PackageName::as_str),
            Some("my-api")
        );
    }

    #[test]
    fn filter_selects_a_single_output() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            package: None,
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
    fn oas_version_check_passes_when_every_partial_matches() {
        // The fake partial source serves OAS 3.0 documents, so a
        // metadata declaration pinning "3.0" is consistent.
        let fixture = fixture_with(Some(OasVersionCheck::V3_0));
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            package: None,
            filter: None,
        };

        let report = interactor.run(&params).unwrap();

        assert_eq!(report.written.len(), 2);
    }

    #[test]
    fn oas_version_mismatch_fails_before_collecting_schemas() {
        // Pinning "3.1" contradicts the 3.0 partials the fake source
        // serves: the run must fail on the first output, without
        // collecting schemas or writing anything.
        let fixture = fixture_with(Some(OasVersionCheck::V3_1));
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            package: None,
            filter: None,
        };

        let result = interactor.run(&params);

        assert!(
            matches!(
                &result,
                Err(Error::OasVersionMismatch {
                    output,
                    partial_version,
                    expected,
                    ..
                }) if output.as_str() == "public"
                    && partial_version == "3.0.3"
                    && *expected == OasVersionCheck::V3_1
            ),
            "expected the version mismatch to be rejected, got {:?}",
            result.map(|report| report.written)
        );
        assert_eq!(*interactor.schemas.calls.borrow(), 0);
        assert!(interactor.outputs.persisted.borrow().is_empty());
    }

    #[test]
    fn unknown_filter_fails_before_writing_anything() {
        let fixture = fixture();
        let interactor = interactor(&fixture);
        let params = GenerateOasParams {
            package: None,
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
