//! Proc-macro derives for frieze.
//!
//! Currently exposes `#[derive(Schema)]`, which generates an implementation
//! of the `frieze::Schema` trait for a named struct whose fields are all of
//! a small fixed scalar set (see [`parse_field_type`]), optionally wrapped
//! in `Option<T>`. Any other shape produces a compile error.
//!
//! The expansion routes every reference to the supporting crates through the
//! `frieze::__private` module so downstream users only need to depend on the
//! `frieze` facade crate.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type};

/// Derive `frieze::Schema` for a named struct whose fields are scalars
/// supported by Phase 1 (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`,
/// `String`), optionally wrapped in `Option<T>` for nullable fields.
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
        let (property_type_expr, optional) = parse_field_type(&field.ty)?;
        property_exprs.push(quote! {
            ::frieze::__private::frieze_model::Property::new(#field_name, #property_type_expr, #optional)
                .expect("frieze: property name is non-empty and derived from a struct field")
        });
    }

    let schema_name = ident.to_string();
    let expanded = quote! {
        impl ::frieze::__private::frieze_usecase::Schema for #ident {
            fn name() -> &'static str {
                #schema_name
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new(
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

/// Maps a Rust field type to the (`PropertyType` expression, optional flag)
/// pair used by the derive expansion.
///
/// Accepts either a directly-supported scalar (see [`property_type_expr`])
/// or a single layer of `Option<T>` wrapping such a scalar. Nested options
/// (`Option<Option<T>>`) and options of unsupported types both produce
/// targeted compile errors.
fn parse_field_type(ty: &Type) -> Result<(proc_macro2::TokenStream, bool), syn::Error> {
    if let Some(inner) = unwrap_option(ty) {
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Option is not supported.",
            ));
        }
        let pt = property_type_expr(inner).map_err(|_| {
            syn::Error::new_spanned(
                ty,
                format!(
                    "frieze: unsupported field type `{}` inside Option<...>; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String. Future PRs will add more.",
                    type_to_display(inner)
                ),
            )
        })?;
        Ok((pt, true))
    } else {
        let pt = property_type_expr(ty)?;
        Ok((pt, false))
    }
}

/// Maps a Rust field type to the matching `frieze_model::PropertyType`,
/// emitted as a path that resolves through the facade re-export.
///
/// Anything other than the small fixed scalar set listed in the error
/// message produces a compile error pointing at the field's type.
fn property_type_expr(ty: &Type) -> Result<proc_macro2::TokenStream, syn::Error> {
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
        other => Err(syn::Error::new_spanned(
            ty,
            format!(
                "frieze: unsupported field type `{other}`; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String. Future PRs will add more."
            ),
        )),
    }
}

/// If `ty` syntactically names `Option<T>` (via `Option`,
/// `std::option::Option`, or `::std::option::Option`), returns `T`.
fn unwrap_option(ty: &Type) -> Option<&Type> {
    let path = match ty {
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => return None,
    };

    // Accept any path whose final segment is `Option`, provided the path
    // is either bare (`Option`) or prefixed with the canonical
    // `std::option` / `core::option` modules. This matches the same shapes
    // `Option<T>`, `std::option::Option<T>`, and `::core::option::Option<T>`.
    let segments: Vec<&syn::PathSegment> = path.segments.iter().collect();
    let recognised = matches!(
        segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .as_slice(),
        [last] if last == "Option"
    ) || matches!(
        segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .as_slice(),
        [first, second, last]
            if (first == "std" || first == "core") && second == "option" && last == "Option"
    );
    if !recognised {
        return None;
    }

    let last = segments.last()?;
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
fn type_to_display(ty: &Type) -> String {
    let rendered = quote! { #ty }.to_string();
    rendered.split_whitespace().collect()
}
