//! A validated, non-empty schema name.

use crate::error::Error;

/// A non-empty schema name.
///
/// Constructed via [`SchemaName::new`]; the inner string is private so the
/// "non-empty" invariant cannot be bypassed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaName(String);

impl SchemaName {
    /// Builds a schema name, rejecting empty input.
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if name.is_empty() {
            return Err(Error::EmptySchemaName);
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
}
