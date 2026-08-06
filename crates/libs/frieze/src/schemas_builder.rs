//! Builder that collects [`Schema`] implementations into a validated
//! [`frieze_model::Schemas`].

use frieze_model::{
    primitive_property_type_for, Error, PropertyType, Schema as ModelSchema, SchemaName, Schemas,
};

use crate::register::{IsRegistrable, Register};

/// In-progress collection of schemas.
///
/// `reserved` holds the first name that collides with a primitive
/// scalar name and surfaces as [`Error::ReservedSchemaName`];
/// `conflict` holds the first same-name / different-content collision
/// and surfaces as [`Error::SchemaConflict`]. Both are observed by
/// [`SchemasBuilder::push_unique`] and reported from
/// [`SchemasBuilder::build`]: the builder follows the same
/// fail-at-finalization pattern as [`Error::UnresolvedReference`] so
/// `register_into` callers and the derive expansion can keep pushing
/// through `push_unique` without threading a `Result` through every
/// registration site.
#[derive(Debug, Default)]
pub struct SchemasBuilder {
    schemas: Vec<ModelSchema>,
    reserved: Option<Error>,
    conflict: Option<Error>,
}

impl SchemasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the schema produced by `T::schema()` and recursively
    /// registers every type that `T`'s schema references.
    ///
    /// `T` must implement [`IsRegistrable`] — this rejects primitive
    /// scalars at compile time (`Schemas::add::<i64>()` fails to
    /// compile), since primitive scalars implement [`crate::Schema`] /
    /// [`Register`] only so they can appear as generic arguments and
    /// are not standalone OAS schema entries. `#[derive(Schema)]` emits
    /// the `IsRegistrable` impl for struct and enum inputs.
    ///
    /// The traversal is performed by [`Register::register_into`]: the
    /// derived impl walks each field type's `register_into`, so a single
    /// `add::<Foo>()` call pulls in `Foo` together with every nested
    /// struct / enum / generic instance reachable from `Foo`'s fields.
    /// Calls are idempotent — adding the same root twice, or having a
    /// root reachable through multiple paths, leaves only one entry per
    /// name in the resulting `Schemas`.
    pub fn add<T: IsRegistrable>(mut self) -> Self {
        <T as Register>::register_into(&mut self);
        self
    }

    /// Pushes `schema` into the in-progress collection only if no schema
    /// with the same registration name is already present.
    ///
    /// `Schema::Scalar` entries (and anything else whose
    /// [`ModelSchema::name`] returns `None`) are always appended — they
    /// have no key to dedup on and are filtered out at
    /// [`Schemas::new`] anyway.
    ///
    /// This is the idempotent push primitive used by
    /// [`Register::register_into`] (both the default impl and the
    /// derive-emitted override) so the same root reached through
    /// multiple paths or via a self-referential cycle (`struct Tree
    /// { children: Vec<Box<Tree>> }`) collapses to a single entry.
    ///
    /// When an incoming schema shares a name with one already registered
    /// but their bodies differ, the first such conflict is recorded on
    /// the builder and reported as [`Error::SchemaConflict`] from
    /// [`Self::build`]. Same-name / same-content pushes remain a silent
    /// dedup — the normal case for a transitive root reached through
    /// multiple paths.
    ///
    /// A name that exactly matches one of the nine primitive scalar
    /// names ([`primitive_property_type_for`]) is likewise recorded and
    /// reported as [`Error::ReservedSchemaName`]. Every registration
    /// path (`add::<T>()`, transitive `register_into`, inventory
    /// collection) funnels through here, so hooking the check at this
    /// point covers all of them. The set is feature-independent:
    /// `Uuid` is reserved even when the `uuid1` feature is off, because
    /// the boundary inlines a `Reference("Uuid")` either way.
    pub fn push_unique(&mut self, schema: ModelSchema) {
        let Some(name) = schema.name().cloned() else {
            // Schemas with no registration name (e.g. `Schema::Scalar`)
            // have no key to dedup on; preserve the historical
            // append-always behaviour. `Schemas::new` filters them out
            // afterwards anyway.
            self.schemas.push(schema);
            return;
        };
        // A reserved name is recorded, but the entry is still pushed:
        // `build` fails regardless, and dropping it here would defeat the
        // `contains_name` guard the derived `register_into` uses to break
        // recursion through a self-referential type.
        if primitive_property_type_for(&name).is_some() && self.reserved.is_none() {
            self.reserved = Some(Error::ReservedSchemaName { name: name.clone() });
        }
        if let Some(existing) = self.schemas.iter().find(|s| s.name() == Some(&name)) {
            if existing != &schema && self.conflict.is_none() {
                self.conflict = Some(Error::SchemaConflict {
                    name,
                    existing: Box::new(existing.clone()),
                    incoming: Box::new(schema),
                });
            }
            // Either way, do not push: a same-name entry is already
            // present, and recording a second copy would defeat the
            // dedup invariant `register_into` relies on.
            return;
        }
        self.schemas.push(schema);
    }

    /// Returns `true` if a previously-pushed schema has the same
    /// registration name as `name`.
    ///
    /// The derive-emitted [`Register::register_into`] uses this as the
    /// early-return guard at the top of the body: `if
    /// builder.contains_name(&Self::name()) { return; }` short-circuits
    /// recursion through self-referential types and multi-path arrival
    /// of the same root.
    pub fn contains_name(&self, name: &str) -> bool {
        self.schemas
            .iter()
            .any(|s| s.name().map(|n| n.as_str() == name).unwrap_or(false))
    }

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

/// Confirms each `oneOf` variant's inner reference resolves to a
/// [`ModelSchema::Object`]. Pointing a oneOf arm at a string-enum or
/// another oneOf would break the internal-tagged shape — the synthesized
/// tag field has nothing to merge into.
fn check_one_of_variants_target_struct_schemas(
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
fn first_unresolved_in_schema<'a>(
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
    use super::*;
    use crate::schema::Schema;
    use frieze_model::{Error, Presence, Property, PropertyType, SchemaName};

    /// `DummyUser` and `DummyProfile` deliberately leave their
    /// `impl Register` blocks empty: the default `register_into` pushes
    /// only `Self`, so these hand-written impls exercise the
    /// non-recursive default path and let us assert the low-level
    /// behaviour (silent dedup, unresolved-reference detection) without
    /// depending on the derive.
    struct DummyUser;

    impl Schema for DummyUser {
        fn name() -> String {
            "User".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "User",
                vec![
                    Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                    Property::new("name", PropertyType::String, Presence::Required).unwrap(),
                ],
            )
            .unwrap()
        }
    }
    impl Register for DummyUser {}
    impl IsRegistrable for DummyUser {}

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

    struct DummyProfile;

    impl Schema for DummyProfile {
        fn name() -> String {
            "Profile".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "Profile",
                vec![Property::new(
                    "user",
                    PropertyType::Reference(SchemaName::new("User").unwrap()),
                    Presence::Required,
                )
                .unwrap()],
            )
            .unwrap()
        }
    }
    impl Register for DummyProfile {}
    impl IsRegistrable for DummyProfile {}

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

    /// Same registration name as `DummyUser` ("User") but with a
    /// different property set, used to exercise the same-name /
    /// different-content path in [`SchemasBuilder::push_unique`].
    struct DummyUserAlt;

    impl Schema for DummyUserAlt {
        fn name() -> String {
            "User".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "User",
                vec![
                    Property::new("id", PropertyType::Int64, Presence::Required).unwrap(),
                    // `DummyUser` has `name: String`; we substitute
                    // `email: String` so the schemas share a name but
                    // differ in content.
                    Property::new("email", PropertyType::String, Presence::Required).unwrap(),
                ],
            )
            .unwrap()
        }
    }
    impl Register for DummyUserAlt {}
    impl IsRegistrable for DummyUserAlt {}

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

    /// Registered under the reserved primitive name `Int64`, which the
    /// boundary conversion inlines as the scalar shape at every
    /// reference position. Registering it must be rejected instead of
    /// leaving the entry silently unreferenced.
    struct DummyInt64;

    impl Schema for DummyInt64 {
        fn name() -> String {
            "Int64".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "Int64",
                vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
            )
            .unwrap()
        }
    }
    impl Register for DummyInt64 {}
    impl IsRegistrable for DummyInt64 {}

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

    /// Registered under the reserved primitive name `Uuid`. The struct
    /// is deliberately **not** behind `#[cfg(feature = "uuid1")]`: the
    /// reservation comes from `primitive_property_type_for`, which the
    /// boundary consults regardless of the feature, so a build without
    /// `uuid1` must reject the name just as firmly.
    struct DummyUuid;

    impl Schema for DummyUuid {
        fn name() -> String {
            "Uuid".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "Uuid",
                vec![Property::new("value", PropertyType::String, Presence::Required).unwrap()],
            )
            .unwrap()
        }
    }
    impl Register for DummyUuid {}
    impl IsRegistrable for DummyUuid {}

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

    /// Composed generic name that merely *starts* with a reserved name
    /// (`Container<i64>` registers as `Int64_Container`). Only exact
    /// matches are reserved, so this must keep building.
    struct DummyInt64Container;

    impl Schema for DummyInt64Container {
        fn name() -> String {
            "Int64_Container".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "Int64_Container",
                vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
            )
            .unwrap()
        }
    }
    impl Register for DummyInt64Container {}
    impl IsRegistrable for DummyInt64Container {}

    /// Namespaced name whose bare segment is a reserved name
    /// (`#[frieze(namespace)] mod v1 { struct Int64 }`). A
    /// namespaced name always carries a dot, so it can never equal a
    /// reserved bare name and stays acceptable.
    struct DummyNamespacedInt64;

    impl Schema for DummyNamespacedInt64 {
        fn name() -> String {
            "v1.Int64".to_string()
        }
        fn schema() -> frieze_model::Schema {
            frieze_model::Schema::new_object(
                "v1.Int64",
                vec![Property::new("value", PropertyType::Int64, Presence::Required).unwrap()],
            )
            .unwrap()
        }
    }
    impl Register for DummyNamespacedInt64 {}
    impl IsRegistrable for DummyNamespacedInt64 {}

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
