//! Expansion for `#[derive(Schema)]` on a named struct.
//!
//! Walks each field once, classifying it via [`crate::field::parse_field`]
//! and computing its wire name through [`crate::rename::wire_name`], then
//! emits the `impl Schema` that constructs a `Schema::new_object(...)` at
//! runtime.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::doc::{description_token, parse_doc_attrs};
use crate::field::{named_fields, parse_field};
use crate::rename::{
    check_unique_wire_names, rename_all_from_scan, wire_name, RenameTarget, WireSource,
};
use crate::serde_scan::{scan_serde_attrs, SerdePosition};

pub(crate) fn expand_struct(ast: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = &ast.ident;
    let container_scan = scan_serde_attrs(&ast.attrs, SerdePosition::StructContainer)?;
    let container_rule = rename_all_from_scan(&container_scan)?;

    let fields = named_fields(ast)?;
    if fields.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: struct must have at least one named field.",
        ));
    }

    let mut property_exprs = Vec::with_capacity(fields.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> = Vec::with_capacity(fields.len());
    for field in fields {
        let field_ident = field
            .ident
            .as_ref()
            .expect("named field has an identifier")
            .clone();
        let field_scan = scan_serde_attrs(&field.attrs, SerdePosition::StructField)?;
        let individual = field_scan.rename.as_ref().map(|(s, sp)| (s.as_str(), *sp));
        let (wire, source) = wire_name(
            &field_ident,
            individual,
            container_rule,
            RenameTarget::Field,
        )?;
        let (property_type_expr, presence_expr) = parse_field(field, &field_scan)?;
        let field_description_expr = description_token(&parse_doc_attrs(&field.attrs));
        let wire_lit = wire.clone();
        property_exprs.push(quote! {
            ::frieze::__private::frieze_model::Property::new(
                #wire_lit,
                #property_type_expr,
                #presence_expr,
            )
            .expect("frieze: property name is non-empty by construction")
            .with_description(#field_description_expr)
        });
        wire_entries.push((field_ident, wire, source));
    }

    check_unique_wire_names(&wire_entries, "field")?;

    let schema_name = ident.to_string();
    let struct_description_expr = description_token(&parse_doc_attrs(&ast.attrs));
    let expanded = quote! {
        impl ::frieze::__private::frieze_usecase::Schema for #ident {
            fn name() -> &'static str {
                #schema_name
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new_object(
                    #schema_name,
                    ::std::vec![ #( #property_exprs ),* ],
                )
                .expect("frieze: derived schema satisfies invariants by construction")
                .with_description(#struct_description_expr)
            }
        }
    };
    Ok(expanded)
}
