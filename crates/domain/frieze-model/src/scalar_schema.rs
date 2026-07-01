//! A validated scalar schema: a single leaf [`PropertyType`] (one of the
//! primitive scalar variants) wrapped in a schema-shaped value so that the
//! [`crate::Schema`] sum can carry primitive scalars alongside the existing
//! object / string-enum / oneOf variants.
//!
//! Scalar schemas exist to let primitive Rust types (`i32`, `i64`, `u32`,
//! `u64`, `f32`, `f64`, `bool`, `String`) implement the
//! `frieze_usecase::Schema` trait so they can appear as generic arguments
//! (`Box<i64>`, `Page<String>`) without forcing a wrapper struct. They are
//! intentionally **not** registered under `#/components/schemas`: the
//! `IsRegistrable` marker trait in `frieze-usecase` is the primary guard
//! that rejects `Schemas::add::<i64>()` at compile time, and the
//! `Scalar` arm in the boundary conversion (`frieze-usecase::to_value`)
//! provides the defensive secondary skip.

use serde::{Deserialize, Serialize};

use crate::description::normalize_description;
use crate::error::Error;
use crate::property_type::PropertyType;

/// A validated scalar schema. The wrapped [`PropertyType`] is restricted
/// to leaf scalar variants — composite variants (`Array`, `Nullable`,
/// `Reference`) are rejected by [`ScalarSchema::new`].
///
/// Fields are private because the only valid construction path is the
/// constructor: a `pub` field would let callers build a `ScalarSchema`
/// holding `PropertyType::Reference`, which violates the leaf-only
/// invariant the consuming side relies on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ScalarSchemaRaw", into = "ScalarSchemaRaw")]
pub struct ScalarSchema {
    property_type: PropertyType,
    description: Option<String>,
}

/// Wire-shape twin of [`ScalarSchema`].
///
/// Serde-derived so the outer [`ScalarSchema`] can keep its
/// leaf-only invariant on the private `property_type` field: the
/// deserialize path funnels through [`ScalarSchema::new`] via
/// [`TryFrom`], rejecting composite variants exactly as the public
/// constructor does. On the serialize side, the twin captures a
/// clone of the outer fields — [`ScalarSchema`] is not consumed
/// during rendering.
#[derive(Serialize, Deserialize)]
struct ScalarSchemaRaw {
    property_type: PropertyType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl From<ScalarSchema> for ScalarSchemaRaw {
    fn from(schema: ScalarSchema) -> Self {
        Self {
            property_type: schema.property_type,
            description: schema.description,
        }
    }
}

impl TryFrom<ScalarSchemaRaw> for ScalarSchema {
    type Error = Error;

    fn try_from(raw: ScalarSchemaRaw) -> Result<Self, Self::Error> {
        Ok(Self::new(raw.property_type)?.with_description(raw.description))
    }
}

impl ScalarSchema {
    /// Builds a scalar schema, rejecting composite `PropertyType` variants
    /// (`Array`, `Nullable`, `Reference`). The description is initialized
    /// to `None`; use [`ScalarSchema::with_description`] to attach one.
    pub fn new(property_type: PropertyType) -> Result<Self, Error> {
        match property_type {
            PropertyType::Int32
            | PropertyType::Int64
            | PropertyType::UInt32
            | PropertyType::UInt64
            | PropertyType::Float
            | PropertyType::Double
            | PropertyType::String
            | PropertyType::Boolean => Ok(Self {
                property_type,
                description: None,
            }),
            PropertyType::Array(_) | PropertyType::Nullable(_) | PropertyType::Reference(_) => {
                Err(Error::NonScalarPropertyType)
            }
        }
    }

    /// Returns the wrapped leaf [`PropertyType`].
    pub fn property_type(&self) -> &PropertyType {
        &self.property_type
    }

    /// Returns the attached description, if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Attaches a top-level description, normalizing empty or
    /// whitespace-only input to `None`.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description.and_then(normalize_description);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_name::SchemaName;

    #[test]
    fn accepts_int64() {
        let s = ScalarSchema::new(PropertyType::Int64).unwrap();
        assert_eq!(s.property_type(), &PropertyType::Int64);
        assert_eq!(s.description(), None);
    }

    #[test]
    fn accepts_string() {
        let s = ScalarSchema::new(PropertyType::String).unwrap();
        assert_eq!(s.property_type(), &PropertyType::String);
    }

    #[test]
    fn accepts_every_leaf_scalar() {
        for ty in [
            PropertyType::Int32,
            PropertyType::Int64,
            PropertyType::UInt32,
            PropertyType::UInt64,
            PropertyType::Float,
            PropertyType::Double,
            PropertyType::String,
            PropertyType::Boolean,
        ] {
            let result = ScalarSchema::new(ty.clone());
            assert!(
                result.is_ok(),
                "expected leaf scalar `{ty:?}` to be accepted"
            );
        }
    }

    #[test]
    fn rejects_array() {
        let ty = PropertyType::Array(Box::new(PropertyType::Int64));
        let err = ScalarSchema::new(ty).unwrap_err();
        assert_eq!(err, Error::NonScalarPropertyType);
    }

    #[test]
    fn rejects_nullable() {
        let ty = PropertyType::Nullable(Box::new(PropertyType::Int64));
        let err = ScalarSchema::new(ty).unwrap_err();
        assert_eq!(err, Error::NonScalarPropertyType);
    }

    #[test]
    fn rejects_reference() {
        let name = SchemaName::new("User").unwrap();
        let err = ScalarSchema::new(PropertyType::Reference(name)).unwrap_err();
        assert_eq!(err, Error::NonScalarPropertyType);
    }

    #[test]
    fn with_description_attaches_text() {
        let s = ScalarSchema::new(PropertyType::Int64)
            .unwrap()
            .with_description(Some("a 64-bit signed integer".into()));
        assert_eq!(s.description(), Some("a 64-bit signed integer"));
    }

    #[test]
    fn with_description_normalizes_blank_to_none() {
        let s = ScalarSchema::new(PropertyType::Int64)
            .unwrap()
            .with_description(Some("   \n  ".into()));
        assert_eq!(s.description(), None);
    }
}
