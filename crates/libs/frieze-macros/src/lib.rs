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
//! - `Vec<Vec<T>>` — nested arrays are not modelled in Phase 1.
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
//! The expansion routes every reference to the supporting crates through the
//! `frieze::__private` module so downstream users only need to depend on the
//! `frieze` facade crate.

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput};

use crate::expand_enum::expand_enum;
use crate::expand_struct::expand_struct;

mod doc;
mod expand_enum;
mod expand_struct;
mod field;
mod property_type;
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
