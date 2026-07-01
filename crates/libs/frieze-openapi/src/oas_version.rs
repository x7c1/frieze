//! Runtime enum lifted from the `openapi:` string in an [`crate::OasDocument`].
//!
//! [`OasVersion`] is the major.minor discriminant used by the pieces of
//! the pipeline that need to switch on OAS shape (nullable encoding,
//! `$ref`-sibling handling, etc.). The OpenAPI patch versions (3.0.0 /
//! 3.0.3 / 3.0.5 / 3.1.0 / 3.1.1) are editorial clarifications only —
//! they never change the schema shape. So dispatch on OAS shape only
//! needs to distinguish `3.0.x` from `3.1.x`.
//!
//! The full patch string that the user wrote is preserved verbatim in
//! [`crate::OasDocument::openapi`] (a `String`); `OasVersion` is a
//! lifted view used for shape-dispatch purposes.

use std::fmt;

/// Major.minor OpenAPI Specification version.
///
/// The wire-format patch string is kept separately on
/// [`crate::OasDocument::openapi`]; this enum only records the
/// major.minor discriminant relevant for shape dispatch.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum OasVersion {
    /// OpenAPI Specification 3.0.x.
    V3_0,
    /// OpenAPI Specification 3.1.x.
    V3_1,
}

impl OasVersion {
    /// Canonical patch string for this major.minor.
    ///
    /// Used when a document is constructed programmatically (e.g. via
    /// `from_schemas`), where the caller specifies major.minor and the
    /// library picks a canonical patch. For partial-driven flows
    /// (`compose`), the partial's raw `openapi:` string is preserved
    /// instead — [`OasVersion`] does not carry that patch.
    pub fn openapi_string(self) -> &'static str {
        match self {
            Self::V3_0 => "3.0.3",
            Self::V3_1 => "3.1.0",
        }
    }

    /// Parses an OpenAPI document's `openapi:` field into a major.minor
    /// [`OasVersion`].
    ///
    /// Accepts any patch value in the `3.0.x` or `3.1.x` range (patch is
    /// preserved separately in the source string). Also accepts the
    /// major.minor form without a patch (`3.0`, `3.1`). Empty strings
    /// yield [`OasVersionParseError::Empty`]; anything outside the
    /// supported major.minor range yields
    /// [`OasVersionParseError::Unsupported`].
    pub fn parse_from_openapi(s: &str) -> Result<Self, OasVersionParseError> {
        if s.is_empty() {
            return Err(OasVersionParseError::Empty);
        }
        // Extract the major.minor prefix. Accept the form "3.0", "3.0.3",
        // "3.1.10", etc. Anything without a `.` (e.g. "3", "abc") is
        // unsupported.
        let major_minor = match s.split_once('.') {
            Some((major, rest)) => match rest.split_once('.') {
                Some((minor, _patch)) => format!("{major}.{minor}"),
                None => format!("{major}.{rest}"),
            },
            None => {
                return Err(OasVersionParseError::Unsupported { got: s.to_string() });
            }
        };
        match major_minor.as_str() {
            "3.0" => Ok(Self::V3_0),
            "3.1" => Ok(Self::V3_1),
            _ => Err(OasVersionParseError::Unsupported { got: s.to_string() }),
        }
    }
}

impl Default for OasVersion {
    /// Defaults to [`OasVersion::V3_0`]. Provided so containers that
    /// require `Default` (e.g. `#[serde(default)]` slots) compile without
    /// forcing every construction site to pick a value; explicit
    /// construction is still preferred.
    fn default() -> Self {
        Self::V3_0
    }
}

/// Errors from [`OasVersion::parse_from_openapi`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OasVersionParseError {
    /// The input string is empty.
    Empty,
    /// The input string is present but does not match a supported
    /// major.minor range (`3.0.x` or `3.1.x`).
    Unsupported { got: String },
}

impl fmt::Display for OasVersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("`openapi` field is empty"),
            Self::Unsupported { got } => write!(
                f,
                "unsupported OAS version `{got}`; supported: 3.0.x, 3.1.x",
            ),
        }
    }
}

impl std::error::Error for OasVersionParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_major_minor_3_0() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.0").unwrap(),
            OasVersion::V3_0
        );
    }

    #[test]
    fn parses_3_0_0() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.0.0").unwrap(),
            OasVersion::V3_0
        );
    }

    #[test]
    fn parses_3_0_5() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.0.5").unwrap(),
            OasVersion::V3_0
        );
    }

    #[test]
    fn parses_3_1_0() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.1.0").unwrap(),
            OasVersion::V3_1
        );
    }

    #[test]
    fn parses_3_1_10() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.1.10").unwrap(),
            OasVersion::V3_1
        );
    }

    #[test]
    fn empty_string_is_empty_error() {
        assert_eq!(
            OasVersion::parse_from_openapi("").unwrap_err(),
            OasVersionParseError::Empty
        );
    }

    #[test]
    fn unsupported_major_minor_2_0() {
        assert_eq!(
            OasVersion::parse_from_openapi("2.0").unwrap_err(),
            OasVersionParseError::Unsupported {
                got: "2.0".to_string()
            }
        );
    }

    #[test]
    fn unsupported_major_minor_3_2_0() {
        assert_eq!(
            OasVersion::parse_from_openapi("3.2.0").unwrap_err(),
            OasVersionParseError::Unsupported {
                got: "3.2.0".to_string()
            }
        );
    }

    #[test]
    fn unparseable_input_is_unsupported() {
        assert_eq!(
            OasVersion::parse_from_openapi("abc").unwrap_err(),
            OasVersionParseError::Unsupported {
                got: "abc".to_string()
            }
        );
    }

    #[test]
    fn canonical_patch_for_3_0() {
        assert_eq!(OasVersion::V3_0.openapi_string(), "3.0.3");
    }

    #[test]
    fn canonical_patch_for_3_1() {
        assert_eq!(OasVersion::V3_1.openapi_string(), "3.1.0");
    }
}
