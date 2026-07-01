//! A validated, non-empty property name.

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// A non-empty property name.
///
/// Constructed via [`PropertyName::new`]; the inner string is private so the
/// "non-empty" invariant cannot be bypassed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PropertyName(String);

impl PropertyName {
    /// Builds a property name, rejecting empty input.
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if name.is_empty() {
            return Err(Error::EmptyPropertyName);
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PropertyName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<PropertyName> for String {
    fn from(value: PropertyName) -> Self {
        value.0
    }
}

impl TryFrom<String> for PropertyName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::fmt::Display for PropertyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(PropertyName::new("").unwrap_err(), Error::EmptyPropertyName);
    }

    #[test]
    fn accepts_non_empty() {
        let name = PropertyName::new("id").unwrap();
        assert_eq!(name.as_str(), "id");
    }
}
