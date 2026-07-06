//! The filesystem-backed [`OutputSink`] implementation.

use std::io;

use frieze_model::{OutputFilePath, OutputFormat};
use frieze_openapi::Document;
use frieze_usecase::{Error, OutputSink, OutputWriteCause, Result};

/// Serializes a generated document (YAML or JSON, per output) and
/// writes it to its output path.
///
/// YAML output goes through [`frieze_openapi::to_yaml`] — the same
/// entry point library callers use — so a document generated through
/// this sink is byte-for-byte identical to one rendered by hand from
/// the same inputs. JSON output uses pretty-printed `serde_json` with
/// a trailing newline.
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
        let text = match format {
            OutputFormat::Yaml => frieze_openapi::to_yaml(document),
            OutputFormat::Json => {
                let mut json = serde_json::to_string_pretty(document).map_err(|cause| {
                    output_write(target, OutputWriteCause::SerializeJson(cause))
                })?;
                json.push('\n');
                json
            }
        };
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
