//! Proc-macro derives for frieze.
//!
//! Currently exposes `#[derive(Schema)]`, which generates an implementation
//! of the `frieze_usecase::Schema` trait for a named struct whose fields are
//! all of type `i64` or `String`. Any other shape produces a compile error.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Type};

/// Derive `frieze_usecase::Schema` for a named struct with `i64` / `String`
/// fields.
#[proc_macro_derive(Schema)]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    expand(ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(ast: DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = &ast.ident;

    let fields = named_fields(&ast)?;
    if fields.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: struct must have at least one named field.",
        ));
    }

    let mut property_exprs = Vec::with_capacity(fields.len());
    for field in fields {
        let field_ident = field.ident.as_ref().expect("named field has an identifier");
        let field_name = field_ident.to_string();
        let property_type_expr = property_type_for(&field.ty)?;
        property_exprs.push(quote! {
            ::frieze_model::Property::new(#field_name, #property_type_expr)
                .expect("frieze: property name is non-empty and derived from a struct field")
        });
    }

    let schema_name = ident.to_string();
    let expanded = quote! {
        impl ::frieze_usecase::Schema for #ident {
            fn name() -> &'static str {
                #schema_name
            }
            fn schema() -> ::frieze_model::Schema {
                ::frieze_model::Schema::new(
                    #schema_name,
                    ::std::vec![ #( #property_exprs ),* ],
                )
                .expect("frieze: derived schema satisfies invariants by construction")
            }
        }
    };
    Ok(expanded)
}

fn named_fields(
    ast: &DeriveInput,
) -> Result<&syn::punctuated::Punctuated<Field, syn::Token![,]>, syn::Error> {
    let data_struct = match &ast.data {
        Data::Struct(s) => s,
        Data::Enum(_) => {
            return Err(syn::Error::new_spanned(
                &ast.ident,
                "frieze: #[derive(Schema)] does not support enums in Phase 1. Future PRs will add support.",
            ));
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &ast.ident,
                "frieze: #[derive(Schema)] does not support unions.",
            ));
        }
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

/// Maps a Rust field type to the matching `frieze_model::PropertyType`.
///
/// Anything other than `i64` and `String` produces a compile error pointing at
/// the field's type.
fn property_type_for(ty: &Type) -> Result<proc_macro2::TokenStream, syn::Error> {
    let rendered = type_to_display(ty);
    match rendered.as_str() {
        "i64" => Ok(quote! { ::frieze_model::PropertyType::Int64 }),
        "String" => Ok(quote! { ::frieze_model::PropertyType::String }),
        other => Err(syn::Error::new_spanned(
            ty,
            format!(
                "frieze: unsupported field type `{other}`; only `i64` and `String` are supported in Phase 1. Future PRs will add support."
            ),
        )),
    }
}

/// Renders a type as a compact string for matching against the known scalars.
/// Whitespace is stripped so `:: i64` and `i64` compare equal.
fn type_to_display(ty: &Type) -> String {
    let rendered = quote! { #ty }.to_string();
    rendered.split_whitespace().collect()
}
