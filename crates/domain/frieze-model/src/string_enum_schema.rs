//! A validated string-enum schema: a non-empty name plus a non-empty list
//! of distinct, non-empty variant values.

use crate::error::Error;
use crate::schema_name::SchemaName;

/// A string-enum schema in its validated form: a non-empty name plus at
/// least one variant value, with no duplicate or empty values.
///
/// Values are stored in declaration order (the order passed to
/// [`StringEnumSchema::new`]) — `frieze` deliberately preserves source
/// order rather than sorting alphabetically, matching the on-the-wire
/// representation produced by serde and the `properties` rule for
/// object schemas.
///
/// Validation happens once, in [`StringEnumSchema::new`]. The fields are
/// `pub` because the type's contract is its shape, not behavior:
/// callers may read or (re-)assign fields directly. Maintaining the
/// documented invariants on a value built via struct-literal or
/// post-construction mutation is the caller's responsibility — the
/// constructor is the only place that checks them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEnumSchema {
    pub name: SchemaName,
    pub values: Vec<String>,
}

impl StringEnumSchema {
    /// Builds a string-enum schema, rejecting empty names, empty value
    /// lists, empty value strings, and duplicate values.
    pub fn new(name: impl Into<String>, values: Vec<String>) -> Result<Self, Error> {
        let name = SchemaName::new(name)?;
        if values.is_empty() {
            return Err(Error::NoVariants(name.into_string()));
        }
        let mut seen: Vec<String> = Vec::with_capacity(values.len());
        for value in &values {
            if value.is_empty() {
                return Err(Error::EmptyVariantValue(name.into_string()));
            }
            if seen.iter().any(|existing| existing == value) {
                return Err(Error::DuplicateVariantValue {
                    schema: name.into_string(),
                    value: value.clone(),
                });
            }
            seen.push(value.clone());
        }
        Ok(Self { name, values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        let err = StringEnumSchema::new("", vec!["Active".into()]).unwrap_err();
        assert_eq!(err, Error::EmptySchemaName);
    }

    #[test]
    fn rejects_no_values() {
        let err = StringEnumSchema::new("Status", Vec::<String>::new()).unwrap_err();
        assert_eq!(err, Error::NoVariants("Status".into()));
    }

    #[test]
    fn rejects_empty_value() {
        let err = StringEnumSchema::new("Status", vec!["".into()]).unwrap_err();
        assert_eq!(err, Error::EmptyVariantValue("Status".into()));
    }

    #[test]
    fn rejects_duplicate_values() {
        let err =
            StringEnumSchema::new("Status", vec!["Active".into(), "Active".into()]).unwrap_err();
        assert_eq!(
            err,
            Error::DuplicateVariantValue {
                schema: "Status".into(),
                value: "Active".into(),
            }
        );
    }

    #[test]
    fn preserves_declaration_order() {
        let schema = StringEnumSchema::new(
            "Status",
            vec!["Active".into(), "Inactive".into(), "Banned".into()],
        )
        .unwrap();
        assert_eq!(schema.values, vec!["Active", "Inactive", "Banned"]);
    }
}
