//! The gateway traits: the seams between the use-case layer and
//! external systems (filesystem, cargo).
//!
//! Each trait covers one kind of external interaction the generate
//! flow needs. The use-case layer holds only these abstractions;
//! concrete implementations live in dedicated gateway crates and are
//! injected by the composition root. Signatures deal exclusively in
//! parsed domain types ([`PackageRoot`], [`PartialFilePath`],
//! [`OutputFilePath`], ...) and in-memory documents
//! ([`Document`] / [`Components`]) — never in raw paths, strings, or
//! bytes. Encoding to and from bytes (YAML/JSON, process stdout) is an
//! implementation detail inside each gateway.
//!
//! Implementations map their internal failures into the semantic
//! boundary variants of [`crate::Error`]
//! ([`crate::Error::PackageResolve`], [`crate::Error::MetadataRead`],
//! [`crate::Error::PartialRead`], [`crate::Error::SchemasCollect`],
//! [`crate::Error::OutputWrite`]) at this boundary.

use frieze_model::{
    OutputFilePath, OutputFormat, PackageMetadata, PackageName, PackageRoot, PartialFilePath,
};
use frieze_openapi::{Components, Document};

use crate::Result;

/// Resolves which package a run targets.
pub trait PackageResolver {
    /// Resolves the target package for the invocation, returning the
    /// root directory of the resolved package.
    ///
    /// The invocation's environment (the current directory and the
    /// workspace enclosing it, including any
    /// `[workspace.metadata.frieze]` declaration) is the
    /// implementation's own input — like every gateway, it hides how
    /// that external state is obtained. `package` is the explicitly
    /// requested package name, when the caller passed one; it always
    /// wins over any environment-derived selection.
    ///
    /// Fails with [`crate::Error::PackageResolve`].
    fn resolve(&self, package: Option<&PackageName>) -> Result<PackageRoot>;
}

/// Reads a package's generation configuration.
pub trait MetadataSource {
    /// Reads the `Cargo.toml` under `root`, extracts the
    /// `[package.metadata.frieze]` section, and converts every raw
    /// path / name / format value into its parsed domain type.
    ///
    /// Fails with [`crate::Error::MissingFriezeSection`] /
    /// [`crate::Error::NoOutputsDefined`] when the section or its
    /// outputs are absent, and with [`crate::Error::MetadataRead`] for
    /// read/parse failures.
    fn read(&self, root: &PackageRoot) -> Result<PackageMetadata>;
}

/// Loads a partial OAS document.
pub trait PartialSource {
    /// Parses the partial OAS document at `path` into a [`Document`].
    ///
    /// The document's OAS version is lifted from its `openapi:` field
    /// during parsing. Fails with [`crate::Error::PartialRead`].
    fn load(&self, path: &PartialFilePath) -> Result<Document>;
}

/// Collects the schemas registered by the target crate.
pub trait SchemasCollector {
    /// Builds and runs a scratch binary that links the target crate,
    /// and receives the canonical, version-neutral [`Components`]
    /// dump it emits.
    ///
    /// `root` locates the target package on disk — the scratch crate
    /// references it as a path dependency, which `metadata` alone (a
    /// parsed configuration value) cannot provide.
    ///
    /// Fails with [`crate::Error::SchemasCollect`].
    fn collect(&self, root: &PackageRoot, metadata: &PackageMetadata) -> Result<Components>;
}

/// Persists a generated OAS document.
pub trait OutputSink {
    /// Serializes `document` in `format` and writes it to `target`.
    ///
    /// Serialization happens inside the implementation — the use-case
    /// layer never sees bytes. Fails with
    /// [`crate::Error::OutputWrite`].
    fn persist(
        &self,
        target: &OutputFilePath,
        document: &Document,
        format: OutputFormat,
    ) -> Result<()>;
}
