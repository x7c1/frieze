//! The filesystem-backed [`PartialSource`] implementation.

use std::io;

use frieze_model::{OutputFormat, PartialFilePath};
use frieze_openapi::Document;
use frieze_usecase::{Error, PartialReadCause, PartialSource, Result};

/// Loads a partial OAS document from disk, parsing YAML or JSON
/// according to the path's format (lifted from its extension at
/// configuration time).
///
/// The document's OAS version is lifted from its `openapi:` field
/// during deserialization; a missing, empty, or unsupported version
/// therefore surfaces as a parse failure whose message names the
/// problem (e.g. `unsupported OAS version ...`), wrapped in
/// [`Error::PartialRead`].
#[derive(Debug, Default)]
pub struct FsPartialSource;

impl FsPartialSource {
    pub fn new() -> Self {
        Self
    }
}

impl PartialSource for FsPartialSource {
    fn load(&self, path: &PartialFilePath) -> Result<Document> {
        let text = std::fs::read_to_string(path.as_path())
            .map_err(|error| partial_read(path, read_cause(error)))?;
        parse(&text, path.format()).map_err(|error| partial_read(path, parse_cause(error)))
    }
}

/// Parses the partial's text in the format its extension declared. Raw
/// parse failures stay in the gateway's internal error here; the
/// caller maps them into the semantic boundary cause.
fn parse(text: &str, format: OutputFormat) -> std::result::Result<Document, crate::Error> {
    match format {
        OutputFormat::Yaml => Ok(serde_yaml::from_str(text)?),
        OutputFormat::Json => Ok(serde_json::from_str(text)?),
    }
}

fn read_cause(error: io::Error) -> PartialReadCause {
    match error.kind() {
        io::ErrorKind::NotFound => PartialReadCause::NotFound,
        io::ErrorKind::PermissionDenied => PartialReadCause::PermissionDenied,
        _ => PartialReadCause::Io(error),
    }
}

/// Maps the internal failures of [`parse`] to their semantic causes.
fn parse_cause(error: crate::Error) -> PartialReadCause {
    match error {
        crate::Error::Yaml(cause) => PartialReadCause::YamlParse(cause),
        crate::Error::Json(cause) => PartialReadCause::JsonParse(cause),
        // Reading happened before parsing; only codecs can fail here.
        crate::Error::Io(_) | crate::Error::Toml(_) => {
            unreachable!("partial parsing uses only the YAML/JSON codecs")
        }
    }
}

fn partial_read(path: &PartialFilePath, cause: PartialReadCause) -> Error {
    Error::PartialRead {
        path: path.clone(),
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frieze_openapi::Version;

    fn partial_path(dir: &std::path::Path, name: &str, content: &str) -> PartialFilePath {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        PartialFilePath::try_from_path(&path).unwrap()
    }

    #[test]
    fn loads_a_yaml_partial_and_lifts_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = partial_path(
            dir.path(),
            "partial.yaml",
            "openapi: 3.1.0\ninfo:\n  title: t\n  version: v\n",
        );
        let document = FsPartialSource::new().load(&path).unwrap();
        assert_eq!(document.oas_version, Version::V3_1);
        assert_eq!(document.openapi, "3.1.0");
        assert_eq!(document.info.title, "t");
    }

    #[test]
    fn loads_a_json_partial() {
        let dir = tempfile::tempdir().unwrap();
        let path = partial_path(
            dir.path(),
            "partial.json",
            r#"{"openapi": "3.0.3", "info": {"title": "t", "version": "v"}}"#,
        );
        let document = FsPartialSource::new().load(&path).unwrap();
        assert_eq!(document.oas_version, Version::V3_0);
    }

    #[test]
    fn unsupported_openapi_version_is_a_curated_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = partial_path(
            dir.path(),
            "partial.yaml",
            "openapi: \"2.0\"\ninfo:\n  title: t\n  version: v\n",
        );
        let result = FsPartialSource::new().load(&path);
        match result {
            Err(Error::PartialRead {
                cause: PartialReadCause::YamlParse(cause),
                ..
            }) => {
                let message = cause.to_string();
                assert!(
                    message.contains("unsupported OAS version `2.0`"),
                    "expected the version message, got: {message}"
                );
            }
            other => panic!("expected a YAML parse failure, got {other:?}"),
        }
    }

    #[test]
    fn missing_openapi_version_is_a_curated_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = partial_path(
            dir.path(),
            "partial.yaml",
            "info:\n  title: t\n  version: v\n",
        );
        let result = FsPartialSource::new().load(&path);
        match result {
            Err(Error::PartialRead {
                cause: PartialReadCause::YamlParse(cause),
                ..
            }) => {
                let message = cause.to_string();
                assert!(
                    message.contains("openapi"),
                    "expected the message to name the `openapi` field, got: {message}"
                );
            }
            other => panic!("expected a YAML parse failure, got {other:?}"),
        }
    }

    #[test]
    fn a_file_that_vanished_after_validation_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = partial_path(dir.path(), "partial.yaml", "openapi: 3.0.3\n");
        std::fs::remove_file(path.as_path()).unwrap();
        let result = FsPartialSource::new().load(&path);
        assert!(
            matches!(
                result,
                Err(Error::PartialRead {
                    cause: PartialReadCause::NotFound,
                    ..
                })
            ),
            "expected the vanished file to be NotFound, got {result:?}"
        );
    }
}
