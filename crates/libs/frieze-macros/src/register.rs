//! Shared helpers for emitting the body of
//! `frieze::Schema::register_into` from `#[derive(Schema)]`.
//!
//! The derived `register_into` walks the syntactic field / variant
//! types of the input, peels off the wrapper layers
//! (`Vec<T>` / `Option<T>` / `Maybe<T>`) that do not implement
//! [`Schema`] themselves, and emits a `<#inner as Schema>::register_into(builder)`
//! call for every terminal type. Multi-parameter generics
//! (`Page<Bar>`, `Pair<i32, f64>`, ...) are passed through as whole
//! types so the monomorphic instance's own derived `register_into`
//! handles the deeper transitive walk at runtime.
//!
//! The emitted body opens with an idempotent guard
//! (`if builder.contains_name(&Self::name()) { return; }`) so recursive
//! types (`struct Tree { children: Vec<Box<Tree>> }`) and multi-path
//! arrivals of the same root collapse to a single entry per name.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::ty::{unwrap_maybe, unwrap_option, unwrap_vec};

/// Recursively strip `Vec<...>` / `Option<...>` / `Maybe<...>` layers
/// to find the terminal inner type whose `register_into` should be
/// invoked.
///
/// These three wrappers intentionally lack a `Schema` impl in
/// `frieze-usecase` (`Vec<i64>` has no OAS-level concept, etc.), so
/// the derive must reach past them. Other parametrised types
/// (`Page<T>`, `Box<T>`, ...) implement `Schema` directly and are
/// returned unchanged — `Box<T>` / `Rc<T>` / `Arc<T>` even delegate
/// their `register_into` to the inner type, so the macro never has to
/// special-case them.
fn strip_collection_wrappers(ty: &Type) -> &Type {
    if let Some(inner) = unwrap_vec(ty) {
        return strip_collection_wrappers(inner);
    }
    if let Some(inner) = unwrap_option(ty) {
        return strip_collection_wrappers(inner);
    }
    if let Some(inner) = unwrap_maybe(ty) {
        return strip_collection_wrappers(inner);
    }
    ty
}

/// Build the body of `register_into` for a struct or enum derive.
///
/// `inner_types` is the list of syntactic types reached directly from
/// the input — struct field types, or internally-tagged enum variant
/// inner types. Unit-only enums pass an empty slice and the emitted
/// body simply pushes `Self::schema()` after the idempotent guard.
pub(crate) fn register_into_body(inner_types: &[&Type]) -> TokenStream {
    let calls = inner_types.iter().map(|ty| {
        let target = strip_collection_wrappers(ty);
        quote! {
            <#target as ::frieze::__private::frieze_usecase::Schema>::register_into(builder);
        }
    });
    quote! {
        if builder.contains_name(
            &<Self as ::frieze::__private::frieze_usecase::Schema>::name(),
        ) {
            return;
        }
        builder.push_unique(<Self as ::frieze::__private::frieze_usecase::Schema>::schema());
        #( #calls )*
    }
}
