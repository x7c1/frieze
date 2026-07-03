//! Expansion for `#[frieze(namespace)]` on a `mod` declaration.
//!
//! The attribute macro intentionally never touches the mod's contents:
//! it parses the input as a `syn::ItemMod`, captures the local ident,
//! and re-emits the original `mod` declaration verbatim with a
//! `::frieze::__private::inventory_namespace!("<ident>");` token next
//! to it. The wrapper macro records `module_path!()` (evaluated at the
//! attribute site) as the namespace's parent path, so the full
//! declaration `format!("{}::{}", parent_path, local_name)` can be
//! reconstructed at runtime by
//! `frieze::__private::compose_schema_name`.
//!
//! Keeping the mod body untouched is what makes file-based mod
//! declarations (`pub mod v1;`) work uniformly with inline mod blocks:
//! a proc-macro cannot read the contents of a file-based `mod`, but it
//! does not need to here — the namespace fact is registered through
//! the side channel and the derive output walks `module_path!()` later.
//!
//! Misuse is caught with curated diagnostics:
//!
//! - `#[frieze(namespace)]` on anything that is not a `mod` declaration
//!   surfaces the "can only be applied to `mod` declarations" error.
//! - `#[frieze(namespace = "v1")]` (any attribute argument at all)
//!   surfaces the "takes no arguments" error; the mod's own ident is
//!   the namespace name.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse, ItemMod};

/// Entry point for the `#[frieze(namespace)]` arm of the `frieze`
/// attribute macro dispatched from `crate::frieze`.
pub fn expand_namespace(args: TokenStream, input: TokenStream) -> TokenStream {
    // The attribute takes no arguments: the mod's own ident is the
    // namespace name. Anything inside the parentheses after
    // `namespace` (e.g. `namespace = "v1"`) is rejected so users do
    // not silently believe a name override took effect.
    if !args.is_empty() {
        let args_ts: TokenStream2 = args.into();
        return syn::Error::new_spanned(
            args_ts,
            "frieze: `#[frieze(namespace)]` takes no arguments; the module's \
             own ident is used as the namespace name. Remove the argument and \
             rename the `mod` if a different name is desired.",
        )
        .to_compile_error()
        .into();
    }

    // The attribute is meaningful only on a `mod` declaration. A
    // dedicated `parse` call lets us surface a curated error message
    // instead of `syn`'s generic "expected `mod`" diagnostic when the
    // attribute is mis-applied (e.g. on a struct).
    let input_ts: TokenStream2 = input.into();
    let module: ItemMod = match parse::<ItemMod>(input_ts.clone().into()) {
        Ok(item) => item,
        Err(_) => {
            return syn::Error::new_spanned(
                input_ts,
                "frieze: `#[frieze(namespace)]` can only be applied to `mod` \
                 declarations (`pub mod foo;` or `pub mod foo { ... }`).",
            )
            .to_compile_error()
            .into();
        }
    };

    let local_name = module.ident.to_string();
    let local_name_lit = syn::LitStr::new(&local_name, module.ident.span());
    let expanded = quote! {
        ::frieze::__private::inventory_namespace!(#local_name_lit);
        #module
    };
    expanded.into()
}
