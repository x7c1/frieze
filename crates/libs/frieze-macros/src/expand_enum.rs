//! Expansion for `#[derive(Schema)]` on a unit-variant enum.
//!
//! Rejects empty enums and non-unit variants; for the supported shape
//! the macro emits an `impl Schema` whose `schema()` returns a
//! `StringEnum` variant. Variant wire names follow the container
//! `rename_all` / per-variant `rename` precedence implemented in
//! [`crate::rename::wire_name`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, DeriveInput, Fields, Ident, Variant};

use crate::doc::{compose_enum_description, description_token, parse_doc_attrs};
use crate::rename::{
    check_unique_wire_names, rename_all_from_scan, wire_name, RenameTarget, WireSource,
};
use crate::serde_scan::{scan_serde_attrs, SerdePosition};

/// Expand `#[derive(Schema)]` on a unit-variant enum into an `impl Schema`
/// whose `schema()` returns a `StringEnum` variant.
///
/// Rejects empty enums and non-unit variants. The container-level
/// `#[serde(rename_all = "...")]` is applied to each variant's wire
/// name, and a variant-level `#[serde(rename = "literal")]` overrides
/// the container rule. Wire-name collisions across variants (whether
/// from literal `rename` or from a `rename_all` collapse) raise a
/// compile error.
pub(crate) fn expand_enum(ast: &DeriveInput, data: &DataEnum) -> Result<TokenStream, syn::Error> {
    let ident = &ast.ident;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: enums with no variants cannot be represented as an OAS schema.",
        ));
    }
    let container_scan = scan_serde_attrs(&ast.attrs, SerdePosition::EnumContainer)?;
    let container_rule = rename_all_from_scan(&container_scan)?;

    let mut values: Vec<String> = Vec::with_capacity(data.variants.len());
    let mut variant_descriptions: Vec<(String, Option<String>)> =
        Vec::with_capacity(data.variants.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> =
        Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        validate_variant(variant)?;
        let variant_scan = scan_serde_attrs(&variant.attrs, SerdePosition::EnumVariant)?;
        let individual = variant_scan
            .rename
            .as_ref()
            .map(|(s, sp)| (s.as_str(), *sp));
        let (wire, source) = wire_name(
            &variant.ident,
            individual,
            container_rule,
            RenameTarget::Variant,
        )?;
        values.push(wire.clone());
        variant_descriptions.push((wire.clone(), parse_doc_attrs(&variant.attrs)));
        wire_entries.push((variant.ident.clone(), wire, source));
    }

    check_unique_wire_names(&wire_entries, "variant")?;

    let schema_name = ident.to_string();
    let value_literals = values.iter().map(|v| {
        quote! { ::std::string::String::from(#v) }
    });
    let enum_doc = parse_doc_attrs(&ast.attrs);
    let composed_description = compose_enum_description(enum_doc.as_deref(), &variant_descriptions);
    let composed_description_expr = description_token(&composed_description);
    let expanded = quote! {
        impl ::frieze::__private::frieze_usecase::Schema for #ident {
            fn name() -> &'static str {
                #schema_name
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new_string_enum(
                    #schema_name,
                    ::std::vec![ #( #value_literals ),* ],
                )
                .expect("frieze: derived enum schema satisfies invariants by construction")
                .with_description(#composed_description_expr)
            }
        }
    };
    Ok(expanded)
}

/// Reject tuple variants (`Foo(i64)`) and struct variants (`Foo { x: i64 }`).
/// Only unit variants are allowed for Phase 1; data-carrying variants
/// belong to the `oneOf` system that has not landed yet.
fn validate_variant(variant: &Variant) -> Result<(), syn::Error> {
    match &variant.fields {
        Fields::Unit => Ok(()),
        Fields::Unnamed(_) => Err(syn::Error::new_spanned(
            variant,
            "frieze: tuple variants are not supported; only unit variants can be expressed as an OAS string enum.",
        )),
        Fields::Named(_) => Err(syn::Error::new_spanned(
            variant,
            "frieze: struct variants are not supported; only unit variants can be expressed as an OAS string enum.",
        )),
    }
}
