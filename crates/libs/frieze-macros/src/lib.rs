//! Proc-macro derives for frieze.
//!
//! Currently exposes `#[derive(Schema)]`, which generates an implementation
//! of the `frieze::Schema` trait for a named struct whose fields are all of
//! a small fixed scalar set (see [`parse_field_type`]), optionally wrapped
//! in `Option<T>` and/or a single layer of `Vec<T>`. Any other shape
//! produces a compile error.
//!
//! The expansion routes every reference to the supporting crates through the
//! `frieze::__private` module so downstream users only need to depend on the
//! `frieze` facade crate.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type,
};

/// Derive `frieze::Schema` for a named struct whose fields are scalars
/// supported by Phase 1 (`i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`,
/// `String`), optionally wrapped in `Vec<T>` for array fields and/or
/// `Option<...>` for nullable fields.
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
/// Accepts:
///
/// - a directly-supported scalar (see [`scalar_property_type_expr`]),
/// - a single layer of `Vec<T>` wrapping such a scalar (array field),
/// - a single layer of `Option<T>` wrapping a scalar (nullable scalar),
/// - a single layer of `Option<Vec<T>>` wrapping a scalar (nullable array).
///
/// Compile errors are raised for nested `Option` (`Option<Option<T>>`),
/// nested `Vec` (`Vec<Vec<T>>`), and `Vec<Option<T>>` shapes — the latter
/// because shape-level optionality cannot be expressed with the current
/// `Property` representation.
fn parse_field_type(ty: &Type) -> Result<(proc_macro2::TokenStream, bool), syn::Error> {
    if let Some(inner) = unwrap_option(ty) {
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Option is not supported.",
            ));
        }
        if let Some(vec_inner) = unwrap_vec(inner) {
            reject_vec_inner(ty, vec_inner)?;
            let element = scalar_property_type_expr(vec_inner).map_err(|_| {
                syn::Error::new_spanned(
                    ty,
                    unsupported_inside_message(vec_inner, "Option<Vec<...>>"),
                )
            })?;
            return Ok((array_property_type_expr(element), true));
        }
        let pt = scalar_property_type_expr(inner).map_err(|_| {
            syn::Error::new_spanned(ty, unsupported_inside_message(inner, "Option<...>"))
        })?;
        Ok((pt, true))
    } else if let Some(vec_inner) = unwrap_vec(ty) {
        reject_vec_inner(ty, vec_inner)?;
        let element = scalar_property_type_expr(vec_inner).map_err(|_| {
            syn::Error::new_spanned(ty, unsupported_inside_message(vec_inner, "Vec<...>"))
        })?;
        Ok((array_property_type_expr(element), false))
    } else {
        let pt = scalar_property_type_expr(ty)
            .map_err(|_| syn::Error::new_spanned(ty, unsupported_message(&type_to_display(ty))))?;
        Ok((pt, false))
    }
}

/// Rejects `Vec<Option<...>>` and `Vec<Vec<...>>` with the targeted
/// error messages used by the macro's UI tests.
fn reject_vec_inner(outer: &Type, vec_inner: &Type) -> Result<(), syn::Error> {
    if unwrap_option(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: Vec<Option<T>> is not supported in Phase 1; shape-level optionality requires a Property restructure that will be revisited in a future PR.",
        ));
    }
    if unwrap_vec(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: nested Vec is not supported.",
        ));
    }
    Ok(())
}

/// Maps a Rust scalar field type to the matching
/// `frieze_model::PropertyType`, emitted as a path that resolves through
/// the facade re-export.
///
/// Anything other than the small fixed scalar set listed in
/// [`unsupported_message`] produces a compile error. Callers are
/// responsible for surfacing that error with the context the user typed
/// (e.g. "inside Option<...>"), since this function only sees the inner
/// scalar.
fn scalar_property_type_expr(ty: &Type) -> Result<proc_macro2::TokenStream, syn::Error> {
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
        _ => Err(syn::Error::new_spanned(ty, unsupported_message(&rendered))),
    }
}

/// Wraps a scalar `PropertyType` expression in
/// `PropertyType::Array(Box::new(...))`.
fn array_property_type_expr(element: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        ::frieze::__private::frieze_model::PropertyType::Array(
            ::std::boxed::Box::new(#element)
        )
    }
}

/// Standard "unsupported field type" error message, listing the Phase 1
/// supported shapes.
fn unsupported_message(rendered: &str) -> String {
    format!(
        "frieze: unsupported field type `{rendered}`; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Option<T>, Option<Vec<T>> (for any supported scalar T). Future PRs will add more."
    )
}

/// Variant of [`unsupported_message`] that names the wrapper containing
/// the offending inner type (e.g. `Option<...>` or `Vec<...>`).
fn unsupported_inside_message(inner: &Type, container: &str) -> String {
    format!(
        "frieze: unsupported field type `{}` inside {container}; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Option<T>, Option<Vec<T>> (for any supported scalar T). Future PRs will add more.",
        type_to_display(inner)
    )
}

/// If `ty` syntactically names `Option<T>` (via `Option`,
/// `std::option::Option`, or `::std::option::Option`), returns `T`.
fn unwrap_option(ty: &Type) -> Option<&Type> {
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
fn unwrap_vec(ty: &Type) -> Option<&Type> {
    unwrap_single_generic(
        ty,
        &[&["Vec"], &["std", "vec", "Vec"], &["alloc", "vec", "Vec"]],
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
fn type_to_display(ty: &Type) -> String {
    let rendered = quote! { #ty }.to_string();
    rendered.split_whitespace().collect()
}
