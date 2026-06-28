//! Syntactic helpers for inspecting [`syn::Type`].
//!
//! These helpers recognise a small fixed set of wrapper types (`Option<T>`,
//! `Vec<T>`, `Maybe<T>`) by their identifier path so the rest of the macro
//! can pattern-match against the inner type without re-implementing the
//! path-matching logic in every call site.

use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// If `ty` syntactically names `Option<T>` (via `Option`,
/// `std::option::Option`, or `::std::option::Option`), returns `T`.
pub(crate) fn unwrap_option(ty: &Type) -> Option<&Type> {
    unwrap_single_generic(
        ty,
        &[
            &["Option"],
            &["std", "option", "Option"],
            &["core", "option", "Option"],
        ],
    )
}

/// If `ty` syntactically names `Vec<T>` (via `Vec`, `std::vec::Vec`,
/// `::std::vec::Vec`, or `alloc::vec::Vec`), returns `T`.
pub(crate) fn unwrap_vec(ty: &Type) -> Option<&Type> {
    unwrap_single_generic(
        ty,
        &[&["Vec"], &["std", "vec", "Vec"], &["alloc", "vec", "Vec"]],
    )
}

/// If `ty` syntactically names `Maybe<T>` (via `Maybe`, `frieze::Maybe`,
/// or `frieze_model::Maybe`), returns `T`.
///
/// Plain `Maybe` is the form users get from `use frieze::Maybe;` (the
/// facade re-export); the longer paths cover users who reach into the
/// underlying crates directly.
pub(crate) fn unwrap_maybe(ty: &Type) -> Option<&Type> {
    unwrap_single_generic(
        ty,
        &[&["Maybe"], &["frieze", "Maybe"], &["frieze_model", "Maybe"]],
    )
}

/// Matches any path in `accepted_paths` against `ty` and, on match,
/// returns the single type argument inside its angle brackets.
///
/// Each accepted path is a list of identifier strings; a path matches if
/// its identifier sequence equals one of the candidates (leading `::` is
/// ignored — the matcher works on identifiers only).
fn unwrap_single_generic<'a>(ty: &'a Type, accepted_paths: &[&[&str]]) -> Option<&'a Type> {
    let path = match ty {
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => return None,
    };
    let idents: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let recognised = accepted_paths.iter().any(|candidate| {
        idents.len() == candidate.len() && idents.iter().zip(*candidate).all(|(a, b)| a == b)
    });
    if !recognised {
        return None;
    }
    let last = path.segments.last()?;
    let args = match &last.arguments {
        PathArguments::AngleBracketed(a) => a,
        _ => return None,
    };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

/// Renders a type as a compact string for matching against the known scalars.
/// Whitespace is stripped so `:: i64` and `i64` compare equal.
pub(crate) fn type_to_display(ty: &Type) -> String {
    let rendered = quote! { #ty }.to_string();
    rendered.split_whitespace().collect()
}
