//! Field-level analysis for `#[derive(Schema)]` on named structs.
//!
//! - [`named_fields`] extracts the punctuated list of fields from a
//!   `DeriveInput`, rejecting tuple and unit structs with a clear error.
//! - [`parse_field`] is the decision entry point: given one field and its
//!   pre-computed [`SerdeScan`], it returns the `PropertyType` expression
//!   plus the `Presence` expression for `Property::new`.

use proc_macro2::TokenStream;
use syn::{Data, DeriveInput, Field, Fields};

use crate::property_type::{
    array_property_type_expr, inner_to_property_type_expr, nullable_property_type_expr,
    presence_optional, presence_required, scalar_property_type_expr, unsupported_inside_message,
    vec_element_property_type_expr,
};
use crate::serde_scan::{has_maybe_attribute_pair, is_option_skip_predicate, SerdeScan};
use crate::ty::{unwrap_maybe, unwrap_option, unwrap_vec};

pub(crate) fn named_fields(
    ast: &DeriveInput,
) -> Result<&syn::punctuated::Punctuated<Field, syn::Token![,]>, syn::Error> {
    let data_struct = match &ast.data {
        Data::Struct(s) => s,
        Data::Enum(_) | Data::Union(_) => unreachable!("dispatched in `expand`"),
    };
    match &data_struct.fields {
        Fields::Named(named) => Ok(&named.named),
        Fields::Unnamed(_) => Err(syn::Error::new_spanned(
            &ast.ident,
            "frieze: #[derive(Schema)] requires a struct with named fields; tuple structs are not supported.",
        )),
        Fields::Unit => Err(syn::Error::new_spanned(
            &ast.ident,
            "frieze: #[derive(Schema)] requires a struct with named fields; unit structs are not supported.",
        )),
    }
}

/// Decision entry point: given a struct field, return the
/// `PropertyType` expression and the `Presence` expression that should
/// be passed to `Property::new` in the generated code.
///
/// Recognises (in order): `Maybe<T>`, `Option<T>` (with serde
/// `skip_serializing_if` attribute discrimination), `Vec<T>`, scalar
/// types. See the crate-level docs for the full table.
///
/// The serde attribute scan is performed by the caller (in
/// `expand_struct`) so the wire-name calculation and the DEFER
/// rejection share a single attribute walk.
pub(crate) fn parse_field(
    field: &Field,
    scan: &SerdeScan,
) -> Result<(TokenStream, TokenStream), syn::Error> {
    let ty = &field.ty;

    if let Some(inner) = unwrap_maybe(ty) {
        // Block doubly-defined / nested cases at the Maybe layer.
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: Maybe<Option<T>> is ambiguous; nullability is doubly defined. Use Maybe<T> alone.",
            ));
        }
        if unwrap_maybe(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Maybe is not supported.",
            ));
        }
        validate_maybe_serde_attrs(field, scan)?;
        let element = inner_to_property_type_expr(inner, ty, "Maybe<...>")?;
        Ok((nullable_property_type_expr(element), presence_optional()))
    } else if let Some(inner) = unwrap_option(ty) {
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Option is not supported.",
            ));
        }
        if unwrap_maybe(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: Option<Maybe<T>> is ambiguous; presence is doubly defined. Use Maybe<T> alone.",
            ));
        }
        // Option<Vec<T>> and Option<Vec<Option<T>>> are both rendered as
        // "outer nullable array"; the items' own nullability is
        // independent.
        if let Some(vec_inner) = unwrap_vec(inner) {
            let element = vec_element_property_type_expr(ty, vec_inner)?;
            let array = array_property_type_expr(element);
            return Ok((nullable_property_type_expr(array), presence_required()));
        }
        // Scalar `T` inside Option — branch ② / ③.
        let scalar = scalar_property_type_expr(inner).map_err(|_| {
            syn::Error::new_spanned(ty, unsupported_inside_message(inner, "Option<...>"))
        })?;
        if is_option_skip_predicate(scan) {
            // Branch ③: optional, non-nullable.
            Ok((scalar, presence_optional()))
        } else {
            // Branch ② (serde default): required, nullable.
            Ok((nullable_property_type_expr(scalar), presence_required()))
        }
    } else if let Some(vec_inner) = unwrap_vec(ty) {
        let element = vec_element_property_type_expr(ty, vec_inner)?;
        Ok((array_property_type_expr(element), presence_required()))
    } else {
        // Pass the error through verbatim so the dedicated "qualified
        // paths" / "generic type parameters" messages from
        // `reference_property_type_expr` reach the user. The generic
        // fallback message already lives inside that helper.
        let pt = scalar_property_type_expr(ty)?;
        Ok((pt, presence_required()))
    }
}

/// Validates that a `Maybe<T>` field carries both serde attributes
/// required for the documented optional-and-nullable round-trip:
///
/// - `#[serde(default)]` (bare form), so a missing key deserialises to
///   `Maybe::Missing` via `Maybe::default()`.
/// - `#[serde(skip_serializing_if = "Maybe::is_missing")]`, so a
///   `Maybe::Missing` value is omitted from the serialised output rather
///   than emitted as `null` (which would collide with `Maybe::Null`).
///
/// Without these, missing / null / present collapse to two states on the
/// wire — a silent runtime breakage. The check enforces them at compile
/// time so users get a clear, actionable diagnostic.
///
/// `default = "..."` with a custom path does not satisfy the pair:
/// serde must call `Maybe::default()` (which yields `Maybe::Missing`); a
/// custom default would defeat the three-state mapping. The
/// `skip_serializing_if` value is matched by exact string against
/// `"Maybe::is_missing"`.
///
/// The two attributes themselves are extracted by `scan_serde_attrs`;
/// this validator only inspects the resulting [`SerdeScan`] so a single
/// pass covers both DEFER rejection and the `Maybe<T>` gate.
fn validate_maybe_serde_attrs(field: &Field, scan: &SerdeScan) -> Result<(), syn::Error> {
    if has_maybe_attribute_pair(scan) {
        return Ok(());
    }
    let field_name = field
        .ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_default();
    let msg = if field_name.is_empty() {
        "frieze: `Maybe<T>` field requires both `#[serde(default)]` and `#[serde(skip_serializing_if = \"Maybe::is_missing\")]`. Add: #[serde(default, skip_serializing_if = \"Maybe::is_missing\")]".to_string()
    } else {
        format!(
            "frieze: `Maybe<T>` field `{field_name}` requires both `#[serde(default)]` and `#[serde(skip_serializing_if = \"Maybe::is_missing\")]`. Add: #[serde(default, skip_serializing_if = \"Maybe::is_missing\")]"
        )
    };
    Err(syn::Error::new_spanned(&field.ty, msg))
}
