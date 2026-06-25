//! A validated property attached to a schema.

use crate::error::Error;
use crate::property_name::PropertyName;
use crate::property_type::PropertyType;

/// A property attached to a schema, in its validated form.
///
/// Constructed via [`Property::new`]; the inner fields are private to prevent
/// construction that bypasses validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    name: PropertyName,
    ty: PropertyType,
}

impl Property {
    /// Builds a property, rejecting empty names.
    pub fn new(name: impl Into<String>, ty: PropertyType) -> Result<Self, Error> {
        Ok(Self {
            name: PropertyName::new(name)?,
            ty,
        })
    }

    pub fn name(&self) -> &PropertyName {
        &self.name
    }

    pub fn ty(&self) -> PropertyType {
        self.ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        let err = Property::new("", PropertyType::Int64).unwrap_err();
        assert_eq!(err, Error::EmptyPropertyName);
    }

    #[test]
    fn accepts_named_property() {
        let property = Property::new("id", PropertyType::Int64).unwrap();
        assert_eq!(property.name().as_str(), "id");
        assert_eq!(property.ty(), PropertyType::Int64);
    }
}
