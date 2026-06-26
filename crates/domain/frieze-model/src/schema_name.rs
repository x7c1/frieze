//! A validated schema name.

use crate::error::Error;

/// A validated schema name suitable for use as a key under
/// `#/components/schemas`.
///
/// Constructed via [`SchemaName::new`]; the inner string is private so the
/// invariants cannot be bypassed.
///
/// # Invariants
///
/// - non-empty
/// - matches the OpenAPI component-name pattern `^[a-zA-Z0-9._-]+$`
///
/// The pattern mirrors the OpenAPI Specification's restriction on
/// component map keys; emitting an unrestricted name would produce YAML
/// that valid OAS toolchains would reject downstream.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaName(String);

impl SchemaName {
    /// Builds a schema name, rejecting empty input and input that does not
    /// match `^[a-zA-Z0-9._-]+$`.
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if name.is_empty() {
            return Err(Error::EmptySchemaName);
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Err(Error::InvalidSchemaName(name));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper, returning the underlying `String`.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for SchemaName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SchemaName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(SchemaName::new("").unwrap_err(), Error::EmptySchemaName);
    }

    #[test]
    fn accepts_non_empty() {
        let name = SchemaName::new("User").unwrap();
        assert_eq!(name.as_str(), "User");
    }

    #[test]
    fn accepts_allowed_characters() {
        // Letters, digits, dot, underscore, hyphen — all permitted.
        for input in ["User", "u_s_e_r", "User.v2", "x7c1-User", "A.B_c-9"] {
            assert!(
                SchemaName::new(input).is_ok(),
                "expected `{input}` to be accepted"
            );
        }
    }

    #[test]
    fn rejects_disallowed_characters() {
        for input in ["User Profile", "User/Profile", "Pro$file", "ユーザー", ""] {
            let result = SchemaName::new(input);
            assert!(
                result.is_err(),
                "expected `{input}` to be rejected, got {result:?}"
            );
        }
    }
}
