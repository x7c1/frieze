//! Whether a property is required to appear on the wire or may be omitted.

use serde::{Deserialize, Serialize};

/// Whether a property's key is required to appear in the serialized object.
///
/// `Presence` is the **presence** axis of optionality — it controls the
/// schema's `required` array (`Required` → name is listed; `Optional` →
/// name is omitted from the array, allowing the key to be absent on the
/// wire).
///
/// `Presence` is intentionally **orthogonal** to value-level nullability,
/// which lives on [`crate::PropertyType::Nullable`]. Together they give
/// frieze four independent combinations:
///
/// | Presence   | Nullable | Allowed wire shapes              |
/// |------------|----------|----------------------------------|
/// | `Required` | no       | `{"k": v}` only                  |
/// | `Required` | yes      | `{"k": v}` or `{"k": null}`      |
/// | `Optional` | no       | `{}` or `{"k": v}`               |
/// | `Optional` | yes      | `{}`, `{"k": v}`, or `{"k": null}` |
///
/// The derive in `frieze-macros` chooses a `Presence` based on the Rust
/// field shape plus serde attributes; see the macro crate docs for the
/// mapping table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Presence {
    /// The property key must be present in the serialized object. The
    /// schema lists the field name under `required`.
    Required,
    /// The property key may be omitted from the serialized object. The
    /// schema omits the field name from `required`.
    Optional,
}

impl Presence {
    /// Returns `true` when this is [`Presence::Required`]. Used as the
    /// single decision point for whether a property name is pushed onto
    /// the schema's `required` array.
    pub fn is_required(self) -> bool {
        matches!(self, Presence::Required)
    }
}
