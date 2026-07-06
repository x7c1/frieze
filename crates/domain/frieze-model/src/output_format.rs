//! The serialization format of a generated OAS document.

use crate::config_error::ConfigError;

/// The on-disk serialization format of an OAS document.
///
/// The format is never configured directly: it is lifted from the file
/// extension of the path the user declared (see
/// [`crate::PartialFilePath`] / [`crate::OutputFilePath`]), so the
/// extension is the single source of truth and no
/// "`.yaml` file containing JSON" mismatch can be expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// YAML (`.yaml` / `.yml`).
    Yaml,
    /// JSON (`.json`).
    Json,
}

impl OutputFormat {
    /// The file extensions accepted by [`Self::try_from_extension`],
    /// in documentation order.
    pub const ALLOWED_EXTENSIONS: &'static [&'static str] = &["yaml", "yml", "json"];

    /// Maps a file extension (without the leading dot) to a format.
    ///
    /// `yaml` / `yml` map to [`OutputFormat::Yaml`], `json` maps to
    /// [`OutputFormat::Json`]. Matching is exact (lowercase); anything
    /// else is rejected with
    /// [`ConfigError::UnsupportedFormatExtension`].
    pub fn try_from_extension(ext: &str) -> Result<Self, ConfigError> {
        match ext {
            "yaml" | "yml" => Ok(Self::Yaml),
            "json" => Ok(Self::Json),
            _ => Err(ConfigError::UnsupportedFormatExtension {
                got: ext.to_string(),
                allowed: Self::ALLOWED_EXTENSIONS,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_extensions() {
        assert_eq!(
            OutputFormat::try_from_extension("yaml").unwrap(),
            OutputFormat::Yaml
        );
        assert_eq!(
            OutputFormat::try_from_extension("yml").unwrap(),
            OutputFormat::Yaml
        );
        assert_eq!(
            OutputFormat::try_from_extension("json").unwrap(),
            OutputFormat::Json
        );
    }

    #[test]
    fn rejects_unsupported_extensions() {
        for ext in ["", "txt", "YAML", "toml", "yaml "] {
            let result = OutputFormat::try_from_extension(ext);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::UnsupportedFormatExtension { got, .. }) if got == ext
                ),
                "expected `{ext}` to be rejected, got {result:?}"
            );
        }
    }
}
