//! Reference walks run by [`super::SchemasBuilder::build`] over the
//! finalized [`Schemas`] collection.

use frieze_model::{
    primitive_property_type_for, Error, PropertyType, Schema as ModelSchema, SchemaName, Schemas,
};

/// Confirms each `oneOf` variant's inner reference resolves to a
/// [`ModelSchema::Object`]. Pointing a oneOf arm at a string-enum or
/// another oneOf would break the internal-tagged shape — the synthesized
/// tag field has nothing to merge into.
pub(super) fn check_one_of_variants_target_struct_schemas(
    schema: &ModelSchema,
    schemas: &Schemas,
) -> Result<(), Error> {
    let one_of = match schema {
        ModelSchema::OneOf(o) => o,
        ModelSchema::Object(_) | ModelSchema::StringEnum(_) | ModelSchema::Scalar(_) => {
            return Ok(())
        }
    };
    for variant in &one_of.variants {
        let target = match schemas.by_name.get(&variant.inner) {
            Some(t) => t,
            // Missing references are already caught by
            // `first_unresolved_in_schema`; defensive `continue` so this
            // helper does not double-report.
            None => continue,
        };
        match target {
            ModelSchema::Object(_) => {}
            ModelSchema::StringEnum(_) | ModelSchema::OneOf(_) | ModelSchema::Scalar(_) => {
                return Err(Error::OneOfVariantInnerNotStruct {
                    schema: one_of.name.as_str().to_string(),
                    variant: variant.wire_name.clone(),
                    inner: variant.inner.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Walks a single registered [`ModelSchema`] for references and returns
/// the first one whose target is not registered in `schemas`.
///
/// Variants that carry no references (e.g. string enums, when added) walk
/// nothing and yield `None`.
pub(super) fn first_unresolved_in_schema<'a>(
    schema: &'a ModelSchema,
    schemas: &Schemas,
) -> Option<&'a SchemaName> {
    match schema {
        ModelSchema::Object(object) => {
            for property in object.properties.values() {
                if let Some(missing) = first_unresolved_reference(&property.ty, schemas) {
                    return Some(missing);
                }
            }
            None
        }
        // String-enum schemas carry no property references; nothing to walk.
        ModelSchema::StringEnum(_) => None,
        ModelSchema::OneOf(one_of) => {
            for variant in &one_of.variants {
                if !schemas.by_name.contains_key(&variant.inner) {
                    return Some(&variant.inner);
                }
            }
            None
        }
        // Scalar schemas carry a single leaf property type and no
        // references; nothing to walk. In practice scalar schemas are
        // filtered out before reaching this point (they are never
        // registered under `#/components/schemas`), but the arm is
        // exhaustive for defensive correctness.
        ModelSchema::Scalar(_) => None,
    }
}

/// Returns the first [`PropertyType::Reference`] encountered in `ty`
/// whose name is not registered in `schemas`, walking
/// [`PropertyType::Array`] and [`PropertyType::Nullable`].
///
/// A reference whose name matches one of the nine primitive scalar
/// names ([`primitive_property_type_for`]) is treated as resolved even
/// when no such entry is registered: primitives implement `Schema` (so
/// they can appear as generic arguments) but not `IsRegistrable`, so
/// `Schemas::add::<i64>()` is intentionally rejected at compile time
/// and the name is never in `schemas.by_name`. The boundary conversion
/// inlines the leaf scalar shape at the reference position, so the
/// resulting OAS document has no dangling `$ref` to follow.
fn first_unresolved_reference<'a>(
    ty: &'a PropertyType,
    schemas: &Schemas,
) -> Option<&'a SchemaName> {
    match ty {
        PropertyType::Reference(name) => {
            if schemas.by_name.contains_key(name) || primitive_property_type_for(name).is_some() {
                None
            } else {
                Some(name)
            }
        }
        PropertyType::Array(inner) | PropertyType::Nullable(inner) => {
            first_unresolved_reference(inner, schemas)
        }
        PropertyType::Int32
        | PropertyType::Int64
        | PropertyType::UInt32
        | PropertyType::UInt64
        | PropertyType::Float
        | PropertyType::Double
        | PropertyType::String
        | PropertyType::Boolean
        | PropertyType::Uuid => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::schemas_builder::testing::{DummyProfile, DummyUser};
    use crate::schemas_builder::SchemasBuilder;
    use frieze_model::{Error, SchemaName};

    #[test]
    fn build_resolves_explicit_reference() {
        let schemas = SchemasBuilder::new()
            .add::<DummyProfile>()
            .add::<DummyUser>()
            .build()
            .expect("explicit registration resolves the reference");
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("User").unwrap()));
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("Profile").unwrap()));
    }

    #[test]
    fn build_detects_unresolved_reference_for_manual_impl() {
        // `DummyProfile` keeps the default (non-recursive)
        // `register_into`, so adding only the profile does not pull in
        // `DummyUser`. The reference `Profile.user -> User` therefore
        // dangles and the builder fails fast.
        //
        // This exercises the path where `UnresolvedReference` is still
        // raised: hand-written impls that reference other types but do
        // not override `Register::register_into` to walk their
        // dependencies. Code using `#[derive(Schema)]` never lands here
        // because the derived `register_into` walks each field type.
        let err = SchemasBuilder::new()
            .add::<DummyProfile>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::UnresolvedReference(SchemaName::new("User").unwrap())
        );
    }
}
