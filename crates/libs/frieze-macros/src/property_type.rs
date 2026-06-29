//! Rust type → `frieze_model::PropertyType` token-stream builders.
//!
//! Translation is split into shape-specific helpers:
//! - [`scalar_property_type_expr`] for the leaf scalars (and a fall-through
//!   to [`reference_property_type_expr`] for nested `Schema`-implementing types),
//! - [`vec_element_property_type_expr`] / [`inner_to_property_type_expr`]
//!   for what may appear inside `Vec<...>` / `Maybe<...>`,
//! - [`array_property_type_expr`] / [`nullable_property_type_expr`] for the
//!   `Array` / `Nullable` constructors, and the [`presence_required`] /
//!   [`presence_optional`] token shortcuts for the matching `Presence`
//!   variants.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::ty::{type_to_display, unwrap_maybe, unwrap_option, unwrap_vec};

/// Builds the items expression for `Vec<inner>`, rejecting compositions
/// not allowed inside `Vec` (nested `Vec`, `Maybe`). Allows `Option<T>`
/// inside `Vec` (rendered as `items: { nullable }`).
pub(crate) fn vec_element_property_type_expr(
    outer: &Type,
    vec_inner: &Type,
) -> Result<TokenStream, syn::Error> {
    if unwrap_vec(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: nested Vec is not supported.",
        ));
    }
    if unwrap_maybe(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: Vec<Maybe<T>> is not allowed; array elements are always present on the wire. Use Vec<Option<T>> if elements may be null.",
        ));
    }
    if let Some(opt_inner) = unwrap_option(vec_inner) {
        // Recursive Option<...> inside Vec — block the same ambiguities as
        // at the top level.
        if unwrap_option(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: nested Option is not supported.",
            ));
        }
        if unwrap_maybe(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: Option<Maybe<T>> is ambiguous; presence is doubly defined.",
            ));
        }
        if unwrap_vec(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: nested Vec is not supported.",
            ));
        }
        let scalar = scalar_property_type_expr(opt_inner).map_err(|_| {
            syn::Error::new_spanned(
                outer,
                unsupported_inside_message(opt_inner, "Vec<Option<...>>"),
            )
        })?;
        return Ok(nullable_property_type_expr(scalar));
    }
    scalar_property_type_expr(vec_inner).map_err(|_| {
        syn::Error::new_spanned(outer, unsupported_inside_message(vec_inner, "Vec<...>"))
    })
}

/// Builds the property-type expression for the inner type of `Maybe<inner>`.
/// `Maybe` may wrap a scalar `T` or a `Vec<T>` / `Vec<Option<T>>`.
pub(crate) fn inner_to_property_type_expr(
    inner: &Type,
    outer_for_span: &Type,
    container: &str,
) -> Result<TokenStream, syn::Error> {
    if let Some(vec_inner) = unwrap_vec(inner) {
        let element = vec_element_property_type_expr(outer_for_span, vec_inner)?;
        return Ok(array_property_type_expr(element));
    }
    scalar_property_type_expr(inner).map_err(|_| {
        syn::Error::new_spanned(outer_for_span, unsupported_inside_message(inner, container))
    })
}

/// Maps a Rust scalar field type to the matching
/// `frieze_model::PropertyType`, emitted as a path that resolves through
/// the facade re-export.
///
/// Falls through to [`reference_property_type_expr`] when the type is a
/// single-segment, unparametrised identifier that isn't a known scalar —
/// such an identifier is treated as a nested struct reference and the
/// expansion emits a `PropertyType::Reference` whose name comes from
/// `<T as frieze::Schema>::name()`. The `Schema` bound itself is enforced
/// by rustc (the user sees the trait-bound diagnostic if the type doesn't
/// implement `Schema`).
///
/// Anything else — qualified paths, generic arguments, primitive types
/// not in the supported set — produces a compile error.
pub(crate) fn scalar_property_type_expr(ty: &Type) -> Result<TokenStream, syn::Error> {
    let rendered = type_to_display(ty);
    match rendered.as_str() {
        "i32" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::Int32 }),
        "i64" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::Int64 }),
        "u32" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::UInt32 }),
        "u64" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::UInt64 }),
        "f32" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::Float }),
        "f64" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::Double }),
        "String" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::String }),
        "bool" => Ok(quote! { ::frieze::__private::frieze_model::PropertyType::Boolean }),
        _ => reference_property_type_expr(ty),
    }
}

/// Treats a single-segment path identifier (optionally carrying generic
/// arguments) as a reference to another `Schema`-implementing type, and
/// emits the `PropertyType::Reference` constructor call.
///
/// Rejects:
///
/// - qualified paths (`mymod::User`) — the macro can't reliably resolve
///   them, so we require the user to bring the type into scope.
/// - any other shape (references, tuples, etc.) — falls back to the
///   generic "unsupported field type" error.
///
/// Generic arguments (`Page<User>`, `Box<i64>`) are accepted: the full
/// type token is forwarded into `<#ty as Schema>::name()` so that the
/// composed name (or, for transparent wrappers like `Box<T>`, the inner
/// type's name) is computed at monomorphisation time. The `Schema` bound
/// is enforced naturally by rustc when the type does not implement it.
fn reference_property_type_expr(ty: &Type) -> Result<TokenStream, syn::Error> {
    let path = match ty {
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => {
            return Err(syn::Error::new_spanned(
                ty,
                unsupported_message(&type_to_display(ty)),
            ));
        }
    };
    if path.segments.len() > 1 {
        return Err(syn::Error::new_spanned(
            ty,
            "frieze: qualified paths in field type are not supported. \
             Use a `use` statement to bring the type into scope.",
        ));
    }
    Ok(quote! {
        ::frieze::__private::frieze_model::PropertyType::Reference(
            ::frieze::__private::frieze_model::SchemaName::new(
                <#ty as ::frieze::__private::frieze_usecase::Schema>::name()
            )
            .expect("frieze: referenced schema name violates the OAS component-name pattern")
        )
    })
}

/// Wraps a `PropertyType` expression in `PropertyType::Array(Box::new(...))`.
pub(crate) fn array_property_type_expr(element: TokenStream) -> TokenStream {
    quote! {
        ::frieze::__private::frieze_model::PropertyType::Array(
            ::std::boxed::Box::new(#element)
        )
    }
}

/// Wraps a `PropertyType` expression in `PropertyType::Nullable(Box::new(...))`.
pub(crate) fn nullable_property_type_expr(inner: TokenStream) -> TokenStream {
    quote! {
        ::frieze::__private::frieze_model::PropertyType::Nullable(
            ::std::boxed::Box::new(#inner)
        )
    }
}

pub(crate) fn presence_required() -> TokenStream {
    quote! { ::frieze::__private::frieze_model::Presence::Required }
}

pub(crate) fn presence_optional() -> TokenStream {
    quote! { ::frieze::__private::frieze_model::Presence::Optional }
}

/// Standard "unsupported field type" error message, listing the Phase 1
/// supported shapes.
fn unsupported_message(rendered: &str) -> String {
    format!(
        "frieze: unsupported field type `{rendered}`; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Vec<Option<T>>, Option<T>, Option<Vec<T>>, Maybe<T> (for any supported scalar T). Future PRs will add more."
    )
}

/// Variant of [`unsupported_message`] that names the wrapper containing
/// the offending inner type (e.g. `Option<...>` / `Vec<...>` / `Maybe<...>`).
pub(crate) fn unsupported_inside_message(inner: &Type, container: &str) -> String {
    format!(
        "frieze: unsupported field type `{}` inside {container}; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Vec<Option<T>>, Option<T>, Option<Vec<T>>, Maybe<T> (for any supported scalar T). Future PRs will add more.",
        type_to_display(inner)
    )
}
