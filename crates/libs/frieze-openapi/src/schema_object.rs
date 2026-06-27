//! The OpenAPI Schema Object subset supported by Phase 1.

use indexmap::IndexMap;

use crate::schema_type::SchemaType;

/// A subset of the OpenAPI Schema Object sufficient for Phase 1.
///
/// `properties` uses [`IndexMap`] to preserve declaration order.
///
/// The field order in this struct mirrors the canonical YAML output order
/// (`$ref`, `type`, `items`, `format`, `minimum`, `allOf`, `oneOf`,
/// `nullable`, `properties`, `required`) so contributors can predict the
/// output shape by reading the struct. Emission itself is performed by the
/// custom emitter `schema_object_to_value` in `frieze-usecase` — serde no
/// longer participates.
///
/// A schema object set to a `$ref` is, per OAS, a leaf — when `reference`
/// is set, callers must not also set sibling fields. The renderer in
/// `frieze-usecase` honours this by emitting `$ref` alone when present.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchemaObject {
    /// JSON pointer to another schema (typically
    /// `#/components/schemas/<name>`). When set, the schema object is a
    /// pure reference: any sibling fields are ignored on the wire.
    pub reference: Option<String>,
    pub ty: Option<SchemaType>,
    /// Element schema for array types. Boxed because [`SchemaObject`]
    /// references itself recursively here (an array's items are themselves
    /// schema objects).
    pub items: Option<Box<SchemaObject>>,
    pub format: Option<String>,
    /// Inclusive lower bound for numeric values. Currently used only to
    /// encode Rust's unsigned semantics in OAS (`minimum: 0` for `u32` /
    /// `u64`), since OAS 3.0 has no canonical unsigned representation.
    ///
    /// Stored as `f64` so the type can represent fractional bounds in the
    /// future. Whole-number values are emitted as YAML integers (e.g. `0`
    /// rather than `0.0`); see `to_value` in `frieze-usecase` for the
    /// emission detail.
    pub minimum: Option<f64>,
    /// `allOf` composition. Used under OAS 3.0 to express
    /// "nullable reference" (`allOf: [$ref], nullable: true`).
    pub all_of: Option<Vec<SchemaObject>>,
    /// `oneOf` composition. Used under OAS 3.1 to express
    /// "nullable reference" (`oneOf: [$ref, {type: "null"}]`).
    pub one_of: Option<Vec<SchemaObject>>,
    /// Carries the intent that this schema accepts `null` in addition to
    /// values of `ty`. The field exists irrespective of the active OAS
    /// version feature — it stores the intent only. The renderer in
    /// `frieze-usecase` translates this flag into the version-appropriate
    /// YAML shape (`nullable: true` for OAS 3.0; a 2-element `type` array
    /// containing `"null"` for OAS 3.1).
    pub nullable: Option<bool>,
    pub properties: Option<IndexMap<String, SchemaObject>>,
    /// Names of properties that must appear on the wire. Omitted from
    /// the emitted schema entirely when empty (an all-optional struct
    /// renders without a `required` key, rather than `required: []`).
    pub required: Vec<String>,
}

impl SchemaObject {
    /// An empty schema object with no fields set.
    pub fn empty() -> Self {
        Self::default()
    }
}
