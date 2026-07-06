//! The optional OAS-version consistency-check value from package
//! metadata.

use crate::config_error::ConfigError;

/// A validated OAS major.minor version declared in package metadata as
/// a consistency check.
///
/// The OAS version a generated document targets is always lifted from
/// the partial document's `openapi:` field — this value is **not** a
/// version selector. When present, it only pins the expected
/// major.minor version so a mismatch between the metadata declaration
/// and a partial document can be rejected with a clear error.
///
/// This is deliberately a model-local type rather than a reuse of the
/// OAS-representation crate's version enum: `frieze-model` depends on
/// no other frieze crate, so the checked value is stored here in its
/// own validated form. Consumers that need to compare it against a
/// document's version can match on the variants or compare
/// [`Self::as_str`] with the document's major.minor string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OasVersionCheck {
    /// OpenAPI Specification 3.0.x (declared as `"3.0"`).
    V3_0,
    /// OpenAPI Specification 3.1.x (declared as `"3.1"`).
    V3_1,
}

impl OasVersionCheck {
    /// Parses the declared value, accepting exactly `"3.0"` or
    /// `"3.1"`. Anything else — including patch-qualified strings like
    /// `"3.0.3"` — is rejected with
    /// [`ConfigError::OasVersionCheckInvalid`]: the declaration pins a
    /// major.minor line, never a patch release.
    pub fn new(s: &str) -> Result<Self, ConfigError> {
        match s {
            "3.0" => Ok(Self::V3_0),
            "3.1" => Ok(Self::V3_1),
            _ => Err(ConfigError::OasVersionCheckInvalid { got: s.to_string() }),
        }
    }

    /// The canonical major.minor string for this value (`"3.0"` /
    /// `"3.1"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V3_0 => "3.0",
            Self::V3_1 => "3.1",
        }
    }
}

impl std::fmt::Display for OasVersionCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_major_minor_values() {
        assert_eq!(OasVersionCheck::new("3.0").unwrap(), OasVersionCheck::V3_0);
        assert_eq!(OasVersionCheck::new("3.1").unwrap(), OasVersionCheck::V3_1);
    }

    #[test]
    fn rejects_anything_else() {
        for input in ["", "2.0", "3.2", "3.0.3", "3.1.0", " 3.0"] {
            let result = OasVersionCheck::new(input);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::OasVersionCheckInvalid { got }) if got == input
                ),
                "expected `{input}` to be rejected, got {result:?}"
            );
        }
    }
}
