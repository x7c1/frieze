//! Finalization of the in-progress collection: any deferred
//! registration error is reported first, then the reference walks run
//! over the resulting [`Schemas`].

use frieze_model::{Error, Schemas};

use super::reference_checks::{
    check_one_of_variants_target_struct_schemas, first_unresolved_in_schema,
};
use super::SchemasBuilder;

impl SchemasBuilder {
    /// Finalizes the collection, checking that every `$ref` resolves
    /// to a registered schema.
    ///
    /// References are gathered by walking each property's type tree
    /// (recursing into `Array(...)` and `Nullable(...)`) and each
    /// `oneOf` variant's inner reference. The first ref that points at
    /// a schema not in the collection produces
    /// [`Error::UnresolvedReference`], in declaration order.
    ///
    /// Duplicate-name detection still runs at the domain layer via
    /// [`Schemas::new`], but the builder pushes every entry through
    /// [`Self::push_unique`], so [`Error::DuplicateSchema`] is no
    /// longer reachable through the standard `add` path; it remains
    /// the defensive guarantee at the model layer when a `Schemas`
    /// value is built outside the use-case-layer builder.
    ///
    /// In addition, each `oneOf` variant's inner reference must point at
    /// a struct schema (`Schema::Object`); pointing at another enum
    /// schema (`Schema::StringEnum` or `Schema::OneOf`) is rejected
    /// with [`Error::OneOfVariantInnerNotStruct`] because the
    /// synthesized tag field must merge into an object body, not into a
    /// scalar-shaped or already-discriminated value.
    ///
    /// Registration problems recorded by [`Self::push_unique`] surface
    /// here before any other validation runs: once a registration is
    /// known to be wrong the rest of the collection cannot be trusted,
    /// so the `$ref` / `oneOf` checks described above are skipped. A
    /// reserved name ([`Error::ReservedSchemaName`]) is reported ahead
    /// of a same-name / different-content conflict
    /// ([`Error::SchemaConflict`]) — a schema occupying a primitive
    /// scalar's name is the more fundamental misuse.
    pub fn build(mut self) -> Result<Schemas, Error> {
        if let Some(reserved) = self.reserved.take() {
            return Err(reserved);
        }
        if let Some(conflict) = self.conflict.take() {
            return Err(conflict);
        }
        let schemas = Schemas::new(self.schemas)?;
        for schema in schemas.by_name.values() {
            if let Some(missing) = first_unresolved_in_schema(schema, &schemas) {
                return Err(Error::UnresolvedReference(missing.clone()));
            }
            check_one_of_variants_target_struct_schemas(schema, &schemas)?;
        }
        Ok(schemas)
    }
}

#[cfg(test)]
mod tests {
    use super::SchemasBuilder;
    use crate::schemas_builder::testing::{
        DummyDate, DummyDateTime, DummyInt64, DummyInt64Container, DummyNamespacedInt64, DummyUser,
        DummyUserAlt, DummyUuid,
    };
    use frieze_model::{Error, SchemaName};

    #[test]
    fn build_dedups_same_root_silently() {
        // Two `add::<DummyUser>()` calls used to surface
        // `Error::DuplicateSchema`; with transitive `register_into`
        // semantics the same root being reached twice is normal (e.g. a
        // recursive type, or two siblings referencing the same nested
        // struct), so the builder silently keeps one entry per name.
        let schemas = SchemasBuilder::new()
            .add::<DummyUser>()
            .add::<DummyUser>()
            .build()
            .expect("duplicate adds collapse silently to a single entry");
        assert_eq!(schemas.by_name.len(), 1);
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("User").unwrap()));
    }

    #[test]
    fn build_detects_schema_conflict() {
        // Two schemas registered under the same name but with different
        // properties must be reported as `Error::SchemaConflict` at
        // `build()` time, rather than silently dedup-ed (which would hide
        // the bug behind first-wins semantics).
        let err = SchemasBuilder::new()
            .add::<DummyUser>()
            .add::<DummyUserAlt>()
            .build()
            .unwrap_err();
        match err {
            Error::SchemaConflict {
                name,
                existing,
                incoming,
            } => {
                assert_eq!(name.as_str(), "User");
                assert_ne!(existing, incoming);
            }
            other => panic!("expected SchemaConflict, got {:?}", other),
        }
    }

    #[test]
    fn build_rejects_reserved_scalar_name() {
        let err = SchemasBuilder::new()
            .add::<DummyInt64>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::ReservedSchemaName {
                name: SchemaName::new("Int64").unwrap(),
            }
        );
    }

    #[test]
    fn build_rejects_reserved_uuid_name_regardless_of_feature() {
        let err = SchemasBuilder::new()
            .add::<DummyUuid>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::ReservedSchemaName {
                name: SchemaName::new("Uuid").unwrap(),
            }
        );
    }

    #[test]
    fn build_rejects_reserved_date_time_name_regardless_of_feature() {
        let err = SchemasBuilder::new()
            .add::<DummyDateTime>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::ReservedSchemaName {
                name: SchemaName::new("DateTime").unwrap(),
            }
        );
    }

    #[test]
    fn build_rejects_reserved_date_name_regardless_of_feature() {
        let err = SchemasBuilder::new()
            .add::<DummyDate>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::ReservedSchemaName {
                name: SchemaName::new("Date").unwrap(),
            }
        );
    }

    #[test]
    fn build_accepts_names_that_only_resemble_reserved_scalars() {
        let schemas = SchemasBuilder::new()
            .add::<DummyInt64Container>()
            .add::<DummyNamespacedInt64>()
            .build()
            .expect("only an exact match with a reserved scalar name is rejected");
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("Int64_Container").unwrap()));
        assert!(schemas
            .by_name
            .contains_key(&SchemaName::new("v1.Int64").unwrap()));
    }

    #[test]
    fn build_reports_reserved_name_before_schema_conflict() {
        // `DummyUser` / `DummyUserAlt` disagree on `User`; the reserved
        // name is the more fundamental misuse and must win over the
        // conflict.
        let err = SchemasBuilder::new()
            .add::<DummyUser>()
            .add::<DummyUserAlt>()
            .add::<DummyInt64>()
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::ReservedSchemaName {
                name: SchemaName::new("Int64").unwrap(),
            }
        );
    }
}
