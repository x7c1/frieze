//! The filesystem-backed [`OutputSink`] implementation.

use std::io;

use frieze_model::{OutputFilePath, OutputFormat};
use frieze_openapi::Document;
use frieze_usecase::{CheckOutcome, Error, OutputCheckCause, OutputSink, OutputWriteCause, Result};

/// Serializes a generated document (YAML or JSON, per output) and
/// writes it to its output path — or, in check mode, compares the
/// rendering against the file already there.
///
/// YAML output goes through [`frieze_openapi::to_yaml`] — the same
/// entry point library callers use — so a document generated through
/// this sink is byte-for-byte identical to one rendered by hand from
/// the same inputs. JSON output uses pretty-printed `serde_json` with
/// a trailing newline. [`OutputSink::verify`] renders through the
/// same function, so its comparison is exact against what a write
/// would produce.
#[derive(Debug, Default)]
pub struct FsOutputSink;

impl FsOutputSink {
    pub fn new() -> Self {
        Self
    }
}

impl OutputSink for FsOutputSink {
    fn persist(
        &self,
        target: &OutputFilePath,
        document: &Document,
        format: OutputFormat,
    ) -> Result<()> {
        let text = render(document, format)
            .map_err(|cause| output_write(target, OutputWriteCause::SerializeJson(cause)))?;
        // The parent directory was created when the path was validated,
        // but that was potentially much earlier in the run — re-create
        // it so a directory removed in between does not fail the write.
        if let Some(parent) = target
            .as_path()
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .map_err(|cause| output_write(target, OutputWriteCause::ParentDirCreate(cause)))?;
        }
        std::fs::write(target.as_path(), text)
            .map_err(|cause| output_write(target, write_cause(cause)))
    }

    fn verify(
        &self,
        target: &OutputFilePath,
        document: &Document,
        format: OutputFormat,
    ) -> Result<CheckOutcome> {
        let expected = render(document, format)
            .map_err(|cause| output_check(target, OutputCheckCause::SerializeJson(cause)))?;
        match std::fs::read(target.as_path()) {
            Ok(actual) if actual == expected.as_bytes() => Ok(CheckOutcome::UpToDate),
            Ok(_) => Ok(CheckOutcome::Stale),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(CheckOutcome::Missing),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                Err(output_check(target, OutputCheckCause::PermissionDenied))
            }
            Err(error) => Err(output_check(target, OutputCheckCause::Read(error))),
        }
    }
}

/// Renders a document to the exact bytes this sink writes for
/// `format`. Only JSON rendering can fail; YAML is infallible.
fn render(document: &Document, format: OutputFormat) -> serde_json::Result<String> {
    match format {
        OutputFormat::Yaml => Ok(frieze_openapi::to_yaml(document)),
        OutputFormat::Json => {
            let mut json = serde_json::to_string_pretty(document)?;
            json.push('\n');
            Ok(json)
        }
    }
}

fn write_cause(error: io::Error) -> OutputWriteCause {
    match error.kind() {
        io::ErrorKind::PermissionDenied => OutputWriteCause::PermissionDenied,
        _ => OutputWriteCause::Write(error),
    }
}

fn output_write(target: &OutputFilePath, cause: OutputWriteCause) -> Error {
    Error::OutputWrite {
        path: target.clone(),
        cause,
    }
}

fn output_check(target: &OutputFilePath, cause: OutputCheckCause) -> Error {
    Error::OutputCheck {
        path: target.clone(),
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frieze_openapi::{Components, Info, Version};

    fn document() -> Document {
        Document::from_components(
            Info {
                title: "t".to_string(),
                version: "v".to_string(),
                ..Info::default()
            },
            Components::default(),
            Version::V3_0,
        )
    }

    #[test]
    fn yaml_output_matches_to_yaml_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap();
        let document = document();
        FsOutputSink::new()
            .persist(&target, &document, OutputFormat::Yaml)
            .unwrap();
        let written = std::fs::read_to_string(target.as_path()).unwrap();
        assert_eq!(written, frieze_openapi::to_yaml(&document));
    }

    #[test]
    fn json_output_is_pretty_printed_with_a_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("openapi.json")).unwrap();
        let document = document();
        FsOutputSink::new()
            .persist(&target, &document, OutputFormat::Json)
            .unwrap();
        let written = std::fs::read_to_string(target.as_path()).unwrap();
        assert!(written.ends_with("}\n"), "got: {written:?}");
        // The written JSON parses back into the same document.
        let parsed: Document = serde_json::from_str(&written).unwrap();
        assert_eq!(parsed, document);
    }

    #[test]
    fn recreates_a_parent_directory_removed_after_validation() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("nested/openapi.yaml")).unwrap();
        std::fs::remove_dir(dir.path().join("nested")).unwrap();
        FsOutputSink::new()
            .persist(&target, &document(), OutputFormat::Yaml)
            .unwrap();
        assert!(target.as_path().is_file());
    }

    #[test]
    fn verify_reports_a_freshly_persisted_output_as_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let sink = FsOutputSink::new();
        let document = document();
        for (file, format) in [
            ("openapi.yaml", OutputFormat::Yaml),
            ("openapi.json", OutputFormat::Json),
        ] {
            let target = OutputFilePath::try_from_path(dir.path().join(file)).unwrap();
            sink.persist(&target, &document, format).unwrap();
            let outcome = sink.verify(&target, &document, format).unwrap();
            assert_eq!(outcome, CheckOutcome::UpToDate, "for {file}");
        }
    }

    #[test]
    fn verify_reports_a_modified_output_as_stale_without_touching_it() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap();
        std::fs::write(target.as_path(), "tampered content\n").unwrap();
        let outcome = FsOutputSink::new()
            .verify(&target, &document(), OutputFormat::Yaml)
            .unwrap();
        assert_eq!(outcome, CheckOutcome::Stale);
        // Check mode is read-only: the file keeps its bytes.
        let content = std::fs::read_to_string(target.as_path()).unwrap();
        assert_eq!(content, "tampered content\n");
    }

    #[test]
    fn verify_reports_an_absent_output_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap();
        std::fs::remove_file(target.as_path()).ok();
        let outcome = FsOutputSink::new()
            .verify(&target, &document(), OutputFormat::Yaml)
            .unwrap();
        assert_eq!(outcome, CheckOutcome::Missing);
    }

    #[test]
    fn overwrites_an_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let target = OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap();
        std::fs::write(target.as_path(), "stale content\n").unwrap();
        let document = document();
        FsOutputSink::new()
            .persist(&target, &document, OutputFormat::Yaml)
            .unwrap();
        let written = std::fs::read_to_string(target.as_path()).unwrap();
        assert_eq!(written, frieze_openapi::to_yaml(&document));
    }
}
