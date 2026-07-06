//! A validated output name.

use crate::config_error::ConfigError;

/// A validated name identifying one declared output of a package.
///
/// Output names key the per-output configuration entries of a package
/// and are what a caller uses to select a single output (e.g. a future
/// `--output <name>` flag). Constructed via [`OutputName::new`]; the
/// inner string is private so the invariants cannot be bypassed.
///
/// # Invariants
///
/// - non-empty
/// - matches `[a-z0-9_-]+` (lowercase ASCII letters, digits,
///   underscore, hyphen)
///
/// Uniqueness within a package is not a per-name invariant; it is
/// enforced by [`crate::PackageMetadata::new`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutputName(String);

impl OutputName {
    /// Builds an output name, rejecting empty input and input with
    /// characters outside `[a-z0-9_-]`.
    pub fn new(name: impl Into<String>) -> Result<Self, ConfigError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ConfigError::OutputNameEmpty);
        }
        let invalid = name.char_indices().find(|(_, c)| {
            !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-'))
        });
        if let Some((at, ch)) = invalid {
            return Err(ConfigError::OutputNameInvalidChar { got: name, at, ch });
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

impl AsRef<str> for OutputName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OutputName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lowercase_digits_underscore_hyphen() {
        for input in ["default", "public", "v2", "api-v1_internal", "0"] {
            assert!(
                OutputName::new(input).is_ok(),
                "expected `{input}` to be accepted"
            );
        }
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            OutputName::new(""),
            Err(ConfigError::OutputNameEmpty)
        ));
    }

    #[test]
    fn rejects_invalid_characters() {
        for (input, expected_at, expected_ch) in
            [("Public", 0, 'P'), ("a b", 1, ' '), ("v1.2", 2, '.')]
        {
            let result = OutputName::new(input);
            assert!(
                matches!(
                    &result,
                    Err(ConfigError::OutputNameInvalidChar { got, at, ch })
                        if got == input && *at == expected_at && *ch == expected_ch
                ),
                "expected `{input}` to be rejected at {expected_at}, got {result:?}"
            );
        }
    }
}
