//! The OpenAPI Specification version as runtime data.
//!
//! [`Version`] is the major.minor discriminant of the OAS version that
//! a [`crate::Document`] targets. OAS patch releases (3.0.0 / 3.0.3 /
//! 3.1.0 / 3.1.1, ...) contain editorial clarifications only — they
//! never change the schema shape — so anything that needs to switch on
//! OAS shape (nullable encoding, `$ref`-sibling handling, ...) only has
//! to distinguish `3.0.x` from `3.1.x`.
//!
//! The verbatim patch string a user wrote stays on
//! [`crate::Document::openapi`] (a `String`); `Version` is the lifted
//! major.minor view used for dispatch.

use std::fmt;

/// Major.minor OpenAPI Specification version.
///
/// There is deliberately no `Default` impl: the version must always
/// come from explicit input — either the `openapi:` field of a parsed
/// document (via [`Version::parse_from_openapi`]) or an explicit
/// argument at construction time. A silent default would let a
/// document be emitted against a version nobody chose.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Version {
    /// OpenAPI Specification 3.0.x.
    V3_0,
    /// OpenAPI Specification 3.1.x.
    V3_1,
}

impl Version {
    /// The canonical `openapi:` field string for this version.
    ///
    /// Used when a document is constructed programmatically (no source
    /// document to preserve a patch string from): the library picks a
    /// fixed, well-known patch release per major.minor. Flows that
    /// start from a parsed document keep that document's verbatim
    /// `openapi` string instead — [`Version`] never carries a patch.
    pub fn openapi_string(self) -> &'static str {
        match self {
            Self::V3_0 => "3.0.3",
            Self::V3_1 => "3.1.0",
        }
    }

    /// Parses an OpenAPI document's `openapi:` field into a
    /// major.minor [`Version`].
    ///
    /// Accepts the bare major.minor forms (`"3.0"`, `"3.1"`) and any
    /// patch release in the supported range (`"3.0.3"`, `"3.1.10"`,
    /// ...) — the patch part is not interpreted, since patch releases
    /// never change schema shape. An empty string yields
    /// [`VersionParseError::Empty`]; anything outside the supported
    /// major.minor range yields [`VersionParseError::Unsupported`].
    pub fn parse_from_openapi(s: &str) -> Result<Self, VersionParseError> {
        if s.is_empty() {
            return Err(VersionParseError::Empty);
        }
        for (major_minor, version) in [("3.0", Self::V3_0), ("3.1", Self::V3_1)] {
            let Some(rest) = s.strip_prefix(major_minor) else {
                continue;
            };
            if rest.is_empty() {
                return Ok(version);
            }
            if let Some(patch) = rest.strip_prefix('.') {
                if !patch.is_empty() {
                    return Ok(version);
                }
            }
        }
        Err(VersionParseError::Unsupported { got: s.to_string() })
    }
}

/// Errors from [`Version::parse_from_openapi`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VersionParseError {
    /// The `openapi` version string is empty.
    Empty,
    /// The `openapi` version string is present but outside the
    /// supported major.minor range (`3.0.x` / `3.1.x`).
    Unsupported {
        /// The rejected version string, verbatim.
        got: String,
    },
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the `openapi` version string is empty"),
            Self::Unsupported { got } => {
                write!(
                    f,
                    "unsupported OAS version `{got}`; supported: 3.0.x, 3.1.x"
                )
            }
        }
    }
}

impl std::error::Error for VersionParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_major_minor() {
        assert_eq!(Version::parse_from_openapi("3.0").unwrap(), Version::V3_0);
        assert_eq!(Version::parse_from_openapi("3.1").unwrap(), Version::V3_1);
    }

    #[test]
    fn parses_any_patch_in_supported_range() {
        assert_eq!(Version::parse_from_openapi("3.0.0").unwrap(), Version::V3_0);
        assert_eq!(Version::parse_from_openapi("3.0.3").unwrap(), Version::V3_0);
        assert_eq!(
            Version::parse_from_openapi("3.0.99").unwrap(),
            Version::V3_0
        );
        assert_eq!(Version::parse_from_openapi("3.1.0").unwrap(), Version::V3_1);
        assert_eq!(
            Version::parse_from_openapi("3.1.10").unwrap(),
            Version::V3_1
        );
    }

    #[test]
    fn empty_string_is_empty_error() {
        assert_eq!(
            Version::parse_from_openapi("").unwrap_err(),
            VersionParseError::Empty
        );
    }

    #[test]
    fn rejects_versions_outside_supported_range() {
        for got in ["2.0", "3.2.0", "3.10", "4.0.0"] {
            assert_eq!(
                Version::parse_from_openapi(got).unwrap_err(),
                VersionParseError::Unsupported {
                    got: got.to_string()
                },
                "input: {got}"
            );
        }
    }

    #[test]
    fn rejects_garbage_input() {
        for got in ["abc", "3", "v3.0", "3.0-rc1", "3.0."] {
            assert_eq!(
                Version::parse_from_openapi(got).unwrap_err(),
                VersionParseError::Unsupported {
                    got: got.to_string()
                },
                "input: {got}"
            );
        }
    }

    #[test]
    fn canonical_openapi_strings() {
        assert_eq!(Version::V3_0.openapi_string(), "3.0.3");
        assert_eq!(Version::V3_1.openapi_string(), "3.1.0");
    }
}
