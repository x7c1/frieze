//! A validated cargo package name.

use crate::config_error::ConfigError;

/// A validated cargo package name.
///
/// Constructed via [`PackageName::new`]; the inner string is private so
/// the invariants cannot be bypassed.
///
/// # Invariants
///
/// - non-empty
/// - every character is an ASCII letter, digit, `_`, or `-`
/// - the first character is an ASCII letter or `_`
///
/// This mirrors the constraints cargo enforces when creating a new
/// package, which is the population these values come from (the
/// `[package] name` of the crate whose metadata is being read).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageName(String);

impl PackageName {
    /// Builds a package name, rejecting input that violates the
    /// documented invariants.
    pub fn new(name: impl Into<String>) -> Result<Self, ConfigError> {
        let name = name.into();
        let mut chars = name.chars();
        let valid = match chars.next() {
            None => false,
            Some(first) => {
                (first.is_ascii_alphabetic() || first == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            }
        };
        if !valid {
            return Err(ConfigError::PackageNameInvalid { got: name });
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

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_typical_package_names() {
        for input in ["my-api", "frieze", "api_v2", "_private", "A1"] {
            assert!(
                PackageName::new(input).is_ok(),
                "expected `{input}` to be accepted"
            );
        }
    }

    #[test]
    fn rejects_invalid_package_names() {
        for input in ["", "1st-api", "-api", "my api", "api/v1", "日本語"] {
            let result = PackageName::new(input);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::PackageNameInvalid { got }) if got == input
                ),
                "expected `{input}` to be rejected, got {result:?}"
            );
        }
    }
}
