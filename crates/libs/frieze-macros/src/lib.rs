//! Proc-macro derives for frieze.
//!
//! `#[derive(Schema)]` generates an implementation of the `frieze::Schema`
//! trait. Three top-level shapes are supported:
//!
//! - **Named struct** — every field type must come from the small fixed
//!   scalar set, optionally composed with `Vec<T>`, `Option<T>`, and/or
//!   `Maybe<T>`, or be itself a `Schema`-deriving type (rendered as
//!   `$ref`). The presence/nullability mapping is documented in the
//!   table below. Struct derives additionally `impl IsStructSchema`
//!   so they can appear as the inner of an internally-tagged enum
//!   variant.
//! - **Unit-variant enum** — every variant must be a unit variant. The
//!   derive emits a `type: string, enum: [...]` schema whose values are
//!   each variant's wire name, computed from the variant identifier
//!   with any container-level `#[serde(rename_all = "...")]` applied
//!   first and any variant-level `#[serde(rename = "literal")]`
//!   overriding it.
//! - **Internally-tagged enum** (`#[serde(tag = "...")]`) — every
//!   variant must be a newtype wrapping a `Schema`-implementing
//!   struct. The derive emits a `oneOf` schema where each arm is an
//!   `allOf` of the inner struct's `$ref` and a synthesized object
//!   constraining the discriminator property to the variant's wire
//!   name; the enclosing schema carries
//!   `discriminator: {propertyName: <tag>}` (no `mapping` block).
//!
//! Any other shape produces a compile error — see the field-shapes
//! spec for the full table of accepted and rejected forms.
//!
//! # Rust shape → OAS combination
//!
//! The struct mapping is driven by syntactic type recognition plus a
//! small fixed set of serde attributes the macro reads: `rename`,
//! `rename_all`, `default`, `skip_serializing_if`, and (for enum
//! containers) `tag`. Any other `#[serde(...)]` entry that frieze
//! cannot faithfully encode into a single OAS schema (`alias`,
//! `flatten`, `content`, `untagged`, `transparent`,
//! `rename_all_fields`, `with` / `serialize_with` /
//! `deserialize_with`, `from` / `try_from` / `into`, `skip` /
//! `skip_serializing` / `skip_deserializing`, `other`, and the
//! direction-split `rename(serialize = ..., deserialize = ...)` /
//! `rename_all(...)` forms) is a compile error.
//!
//! | Rust shape                                                       | Presence | Nullability        |
//! |------------------------------------------------------------------|----------|--------------------|
//! | `T` (scalar)                                                     | required | non-nullable       |
//! | `Option<T>` (serde default)                                      | required | nullable           |
//! | `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`| optional | non-nullable       |
//! | `Maybe<T>`                                                       | optional | nullable           |
//! | `Vec<T>`                                                         | required | array, items as T  |
//! | `Vec<Option<T>>`                                                 | required | array, items nullable |
//! | `Option<Vec<T>>`                                                 | required | nullable array     |
//! | `Option<Vec<Option<T>>>`                                         | required | nullable array, items nullable |
//!
//! # Rejected shapes (compile error)
//!
//! - `Option<Option<T>>` — serde flattens nested options.
//! - `Vec<Vec<T>>` — nested arrays are not modelled.
//! - `Vec<Maybe<T>>` — array elements cannot be `Missing`; use
//!   `Vec<Option<T>>` for arrays of nullable items.
//! - `Option<Maybe<T>>` — presence is doubly defined.
//! - `Maybe<Option<T>>` — nullability is doubly defined.
//! - `Maybe<Maybe<T>>` — nested `Maybe` is not supported.
//!
//! In addition, a `Maybe<T>` field that is missing either
//! `#[serde(default)]` or `#[serde(skip_serializing_if = "Maybe::is_missing")]`
//! is rejected at compile time: without the pair, the three-state
//! missing / null / present mapping collapses on the wire.
//!
//! The expansion routes every reference through the `frieze::__private`
//! module so downstream users only need to depend on the `frieze` crate.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse, parse_macro_input, Data, DeriveInput, Ident, Meta};

use crate::expand_enum::expand_enum;
use crate::expand_struct::expand_struct;

mod doc;
mod expand_enum;
mod expand_struct;
mod field;
mod namespace_attr;
mod property_type;
mod register;
mod rename;
mod serde_scan;
mod ty;

/// Derive `frieze::Schema`. See the crate-level docs for the supported
/// top-level shapes (named struct, unit-variant enum) and the mapping
/// table for struct fields.
#[proc_macro_derive(Schema)]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    expand(ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// The `#[frieze(...)]` attribute macro.
///
/// Currently the only recognised sub-keyword is `namespace`. The
/// `#[frieze(namespace)]` form is applied to a `mod` declaration to
/// register the module as a namespace for OAS schema-name composition
/// — see [`namespace_attr::expand_namespace`] for the full semantics.
///
/// The attribute path is `#[frieze(...)]` rather than
/// `#[frieze::namespace]` to match serde-style conventions where a
/// single attribute name dispatches on the inner keyword
/// (`#[serde(rename = "...")]`, `#[serde(skip)]`, ...).
#[proc_macro_attribute]
pub fn frieze(args: TokenStream, input: TokenStream) -> TokenStream {
    let args_ts2: TokenStream2 = args.into();
    let (keyword, remainder) = match parse_frieze_keyword(args_ts2.clone()) {
        Ok(parts) => parts,
        Err(err) => return err.to_compile_error().into(),
    };

    match keyword.to_string().as_str() {
        "namespace" => namespace_attr::expand_namespace(remainder.into(), input),
        other => syn::Error::new(
            keyword.span(),
            format!(
                "frieze: unknown sub-keyword `{other}` for `#[frieze(...)]`. \
                 The only currently supported keyword is `namespace`."
            ),
        )
        .to_compile_error()
        .into(),
    }
}

/// Parse the attribute arguments into a leading keyword identifier and
/// the remaining tokens (for the keyword's own arg parser).
///
/// `#[frieze(namespace)]` → keyword `namespace`, remainder empty.
/// `#[frieze(namespace = "v1")]` → keyword `namespace`, remainder
///   `= "v1"`. The remainder is forwarded to the keyword's expander
/// (here `namespace_attr::expand_namespace`), which rejects non-empty
/// remainders with a curated diagnostic.
fn parse_frieze_keyword(args: TokenStream2) -> Result<(Ident, TokenStream2), syn::Error> {
    // Empty `#[frieze]` is rejected up front — the attribute is a
    // dispatch entry point and needs a keyword to know what to do.
    if args.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "frieze: `#[frieze(...)]` requires a sub-keyword (e.g. `#[frieze(namespace)]`).",
        ));
    }

    // Use `syn::Meta` to lift the first token group cleanly: it
    // recognises both `namespace` (Path) and `namespace = ...`
    // (NameValue) shapes uniformly, so the keyword + remainder split
    // is the same code path.
    let meta: Meta = parse(args.clone().into())?;
    match meta {
        Meta::Path(path) => {
            let ident = path.get_ident().cloned().ok_or_else(|| {
                syn::Error::new_spanned(
                    &path,
                    "frieze: `#[frieze(...)]` sub-keyword must be a single identifier.",
                )
            })?;
            Ok((ident, TokenStream2::new()))
        }
        Meta::NameValue(nv) => {
            let ident = nv.path.get_ident().cloned().ok_or_else(|| {
                syn::Error::new_spanned(
                    &nv.path,
                    "frieze: `#[frieze(...)]` sub-keyword must be a single identifier.",
                )
            })?;
            // Reconstruct the `= <value>` portion so the keyword's
            // expander can decide whether to accept or reject it.
            let value = &nv.value;
            let remainder = quote::quote! { = #value };
            Ok((ident, remainder))
        }
        Meta::List(list) => {
            let ident = list.path.get_ident().cloned().ok_or_else(|| {
                syn::Error::new_spanned(
                    &list.path,
                    "frieze: `#[frieze(...)]` sub-keyword must be a single identifier.",
                )
            })?;
            let tokens = list.tokens.clone();
            Ok((ident, tokens))
        }
    }
}

fn expand(ast: DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &ast.data {
        Data::Struct(_) => expand_struct(&ast),
        Data::Enum(data) => expand_enum(&ast, data),
        Data::Union(_) => Err(syn::Error::new_spanned(
            &ast.ident,
            "frieze: #[derive(Schema)] does not support unions.",
        )),
    }
}
