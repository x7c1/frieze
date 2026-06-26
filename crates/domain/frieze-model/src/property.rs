//! A validated property attached to a schema.

use crate::error::Error;
use crate::property_name::PropertyName;
use crate::property_type::PropertyType;

/// A property attached to a schema, in its validated form.
///
/// Validation happens once, in [`Property::new`]. The fields are `pub`
/// because the type's contract is its shape, not behavior: callers may read
/// or (re-)assign fields directly. Maintaining the documented invariants on
/// a value built via struct-literal or post-construction mutation is the
/// caller's responsibility — the constructor is the only place that checks
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: PropertyName,
    /// `ty` follows the Rust AST convention (e.g. `syn::Field::ty`, the `ty`
    /// fragment specifier in `macro_rules!`). `frieze-macros` reads
    /// `syn::Field::ty` to populate this field, so the naming is consistent
    /// across the AST boundary. Kept short and idiomatic rather than
    /// `type_` / `tpe` / `r#type`.
    pub ty: PropertyType,
    /// Whether the field is wrapped in `Option<T>` at the Rust source level.
    ///
    /// Carries the intent that the value is allowed to be absent / null. How
    /// this intent is rendered into OAS YAML depends on the selected OAS
    /// version feature (`nullable: true` for `oas-3-0`, a 2-element `type`
    /// sequence for `oas-3-1`). Also drives whether the field name is
    /// included in the schema's `required` array (optional fields are not).
    pub optional: bool,
}

impl Property {
    /// Builds a property, rejecting empty names.
    pub fn new(name: impl Into<String>, ty: PropertyType, optional: bool) -> Result<Self, Error> {
        Ok(Self {
            name: PropertyName::new(name)?,
            ty,
            optional,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        let err = Property::new("", PropertyType::Int64, false).unwrap_err();
        assert_eq!(err, Error::EmptyPropertyName);
    }

    #[test]
    fn accepts_named_property() {
        let property = Property::new("id", PropertyType::Int64, false).unwrap();
        assert_eq!(property.name.as_str(), "id");
        assert_eq!(property.ty, PropertyType::Int64);
        assert!(!property.optional);
    }

    #[test]
    fn accepts_optional_property() {
        let property = Property::new("nickname", PropertyType::String, true).unwrap();
        assert_eq!(property.name.as_str(), "nickname");
        assert_eq!(property.ty, PropertyType::String);
        assert!(property.optional);
    }
}
