//! A validated cargo feature name.

use crate::config_error::ConfigError;

/// A validated cargo feature name, as declared in package metadata to
/// be enabled on the target crate during schema collection.
///
/// Constructed via [`CargoFeatureName::new`]; the inner string is
/// private so the invariants cannot be bypassed.
///
/// # Invariants
///
/// - non-empty
/// - the first character is an ASCII letter, digit, or `_`
/// - every following character is an ASCII letter, digit, `_`, `-`,
///   `.`, or `+`
///
/// This mirrors the character set cargo accepts for feature names.
/// Dependency-scoped forms (`dep:foo`, `foo/bar`, `foo?/bar`) are
/// deliberately rejected: the metadata declares plain features of the
/// target crate itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CargoFeatureName(String);

impl CargoFeatureName {
    /// Builds a feature name, rejecting input that violates the
    /// documented invariants.
    pub fn new(name: impl Into<String>) -> Result<Self, ConfigError> {
        let name = name.into();
        let mut chars = name.chars();
        let valid = match chars.next() {
            None => false,
            Some(first) => {
                (first.is_ascii_alphanumeric() || first == '_')
                    && chars
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
            }
        };
        if !valid {
            return Err(ConfigError::CargoFeatureNameInvalid { got: name });
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

impl AsRef<str> for CargoFeatureName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CargoFeatureName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_typical_feature_names() {
        for input in [
            "default",
            "json-schema",
            "serde_support",
            "v1.2",
            "simd+avx",
            "8bit",
        ] {
            assert!(
                CargoFeatureName::new(input).is_ok(),
                "expected `{input}` to be accepted"
            );
        }
    }

    #[test]
    fn rejects_invalid_feature_names() {
        for input in ["", "-leading-hyphen", "dep:foo", "foo/bar", "with space"] {
            let result = CargoFeatureName::new(input);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::CargoFeatureNameInvalid { got }) if got == input
                ),
                "expected `{input}` to be rejected, got {result:?}"
            );
        }
    }
}
