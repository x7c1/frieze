//! A three-state value type for "missing / null / present" fields.
//!
//! See [`Maybe`] for the type itself and the conventions for using it on a
//! `serde`-derived struct.

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// A three-state value distinguishing "key missing", "key present with
/// `null`", and "key present with a value".
///
/// `Maybe<T>` exists to express the OpenAPI combination `optional` **and**
/// `nullable` (the schema's `required` array omits the field, **and** the
/// field accepts `null` when present). Plain `Option<T>` cannot do this on
/// its own under serde's default behavior, where a missing field is also
/// `None` and indistinguishable from a `null` value.
///
/// Independently of OAS, the three-state distinction is useful on its own
/// — for example, an HTTP `PATCH` request body needs to tell "unspecified"
/// (`Missing`) apart from "explicitly clear this field" (`Null`).
///
/// # Recommended attributes
///
/// `Maybe<T>` participates in serde via the standard derived
/// [`Serialize`] / [`Deserialize`] impls. The fully-symmetric idiom is:
///
/// ```
/// # use serde::{Serialize, Deserialize};
/// # use frieze_model::Maybe;
/// #[derive(Serialize, Deserialize)]
/// struct Patch {
///     #[serde(default, skip_serializing_if = "Maybe::is_missing")]
///     avatar_url: Maybe<String>,
/// }
/// ```
///
/// - `#[serde(default)]` is required so that a missing key during
///   deserialization yields [`Maybe::Missing`] (which is also the
///   [`Default`] returned by [`Maybe::default`]). Without it, serde rejects
///   the missing field outright.
/// - `#[serde(skip_serializing_if = "Maybe::is_missing")]` is required so
///   that `Missing` causes the key to be **omitted** on the wire. Without
///   it, `Missing` serializes as `null`, collapsing the distinction with
///   [`Maybe::Null`].
///
/// The `#[derive(frieze::Schema)]` derive recognises `Maybe<T>` as the
/// "optional + nullable" combination regardless of which of these
/// attributes the user remembers to add; the attributes are a serde-side
/// concern, not a schema-side concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Maybe<T> {
    /// Key absent from the serialized object.
    Missing,
    /// Key present with a `null` value.
    Null,
    /// Key present with a value of type `T`.
    Present(T),
}

impl<T> Maybe<T> {
    /// Returns `true` when this is [`Maybe::Missing`]. Intended for use as
    /// the predicate in `#[serde(skip_serializing_if = "Maybe::is_missing")]`.
    pub fn is_missing(&self) -> bool {
        matches!(self, Maybe::Missing)
    }

    /// Returns `true` when this is [`Maybe::Null`].
    pub fn is_null(&self) -> bool {
        matches!(self, Maybe::Null)
    }

    /// Returns `true` when this is [`Maybe::Present`].
    pub fn is_present(&self) -> bool {
        matches!(self, Maybe::Present(_))
    }
}

impl<T> Default for Maybe<T> {
    /// Returns [`Maybe::Missing`]. Paired with `#[serde(default)]` so that
    /// a missing field during deserialization round-trips to `Missing`.
    fn default() -> Self {
        Maybe::Missing
    }
}

impl<T> From<Option<T>> for Maybe<T> {
    /// `None` → [`Maybe::Null`]; `Some(v)` → [`Maybe::Present(v)`].
    ///
    /// `Missing` has no counterpart in [`Option`] and is reachable only
    /// through [`Maybe::default`] or explicit construction.
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => Maybe::Null,
            Some(v) => Maybe::Present(v),
        }
    }
}

impl<T: Serialize> Serialize for Maybe<T> {
    /// `Missing` and `Null` both serialize as `null`. The distinction is
    /// preserved on the wire by pairing the field with
    /// `#[serde(skip_serializing_if = "Maybe::is_missing")]`, which causes
    /// the serializer to skip the field entirely when it is `Missing`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Maybe::Missing | Maybe::Null => serializer.serialize_none(),
            Maybe::Present(v) => serializer.serialize_some(v),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Maybe<T> {
    /// Deserializes by delegating to `Option<T>::deserialize`: `null`
    /// becomes [`Maybe::Null`] and any value becomes [`Maybe::Present`].
    ///
    /// A missing struct field is not visible to this impl; serde handles
    /// it by calling [`Default::default`] when the field is annotated
    /// with `#[serde(default)]`, producing [`Maybe::Missing`].
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(Maybe::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_missing() {
        let m: Maybe<i64> = Maybe::default();
        assert_eq!(m, Maybe::Missing);
        assert!(m.is_missing());
    }

    #[test]
    fn predicates_classify_each_variant() {
        let missing: Maybe<i64> = Maybe::Missing;
        let null: Maybe<i64> = Maybe::Null;
        let present: Maybe<i64> = Maybe::Present(42);

        assert!(missing.is_missing());
        assert!(!missing.is_null());
        assert!(!missing.is_present());

        assert!(null.is_null());
        assert!(!null.is_missing());
        assert!(!null.is_present());

        assert!(present.is_present());
        assert!(!present.is_missing());
        assert!(!present.is_null());
    }

    #[test]
    fn from_option_maps_none_to_null_and_some_to_present() {
        assert_eq!(Maybe::<i64>::from(None), Maybe::Null);
        assert_eq!(Maybe::<i64>::from(Some(7)), Maybe::Present(7));
    }
}
