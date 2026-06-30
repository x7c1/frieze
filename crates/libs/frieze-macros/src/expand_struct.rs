//! Expansion for `#[derive(Schema)]` on a named struct.
//!
//! Walks each field once, classifying it via [`crate::field::parse_field`]
//! and computing its wire name through [`crate::rename::wire_name`], then
//! emits the `impl Schema` that constructs a `Schema::new_object(...)` at
//! runtime.
//!
//! # Generic structs
//!
//! When the input carries type parameters (`struct Page<T> { ... }`), the
//! derive propagates them onto the emitted `impl` blocks with a
//! synthesised `T: Schema` bound on every type parameter (alongside the
//! user's own `where` clause, which is preserved verbatim), and the
//! generated `name()` composes the schema name from each parameter's
//! name in the **suffix** form `<Arg1>_<Arg2>_..._<BaseName>` —
//! `Container<i64>` becomes `Int64_Container`, `Pair<i32, f64>` becomes
//! `Int32_Float_Pair`, `Container<Container<i64>>` becomes
//! `Int64_Container_Container`. Non-generic structs keep emitting the
//! plain literal base name.
//!
//! Lifetime parameters and const generics are rejected at compile time:
//! frieze schemas describe owned data layouts, and the OAS representation
//! of either feature is undecided.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, GenericParam, Ident};

use crate::doc::{description_token, parse_doc_attrs};
use crate::field::{named_fields, parse_field};
use crate::register::{inventory_submit_token, register_into_body};
use crate::rename::{
    check_unique_wire_names, rename_all_from_scan, wire_name, RenameTarget, WireSource,
};
use crate::serde_scan::{scan_serde_attrs, SerdePosition};

pub(crate) fn expand_struct(ast: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = &ast.ident;
    let container_scan = scan_serde_attrs(&ast.attrs, SerdePosition::StructContainer)?;
    let container_rule = rename_all_from_scan(&container_scan)?;

    // Reject lifetime / const-generic parameters before doing anything
    // else so we don't waste downstream work on a shape we won't emit.
    reject_unsupported_generic_params(&ast.generics, ident)?;

    let fields = named_fields(ast)?;
    if fields.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: struct must have at least one named field.",
        ));
    }

    let mut property_exprs = Vec::with_capacity(fields.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> = Vec::with_capacity(fields.len());
    let field_types: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();
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

    let base_name = ident.to_string();
    let struct_description_expr = description_token(&parse_doc_attrs(&ast.attrs));

    // Build the impl-generics (`<T: Schema, U: Schema>`), type-generics
    // (`<T, U>`), and `where` clause. We clone `ast.generics` and push a
    // `Schema` bound onto every type parameter so that `Container<T>`
    // requires `T: Schema` automatically. The user's `where` clause is
    // preserved verbatim through `split_for_impl()`.
    let mut impl_generics_storage = ast.generics.clone();
    for param in impl_generics_storage.params.iter_mut() {
        if let GenericParam::Type(type_param) = param {
            let bound: syn::TypeParamBound =
                syn::parse_quote!(::frieze::__private::frieze_usecase::Schema);
            type_param.bounds.push(bound);
        }
    }
    let (impl_generics, ty_generics, where_clause) = impl_generics_storage.split_for_impl();

    let name_body = composed_name_body(&ast.generics, &base_name);
    let register_body = register_into_body(&field_types);
    // Non-generic struct: emit one `inventory::submit!` site so the
    // type appears as a root in `Schemas::builder().from_inventory()`.
    // Generic structs produce an empty token stream — see
    // `inventory_submit_token` for the rationale.
    let inventory_submit = inventory_submit_token(ident, &ast.generics);

    let expanded = quote! {
        impl #impl_generics ::frieze::__private::frieze_usecase::Schema for #ident #ty_generics #where_clause {
            fn name() -> ::std::string::String {
                #name_body
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new_object(
                    <Self as ::frieze::__private::frieze_usecase::Schema>::name(),
                    ::std::vec![ #( #property_exprs ),* ],
                )
                .expect("frieze: derived schema satisfies invariants by construction")
                .with_description(#struct_description_expr)
            }
            fn register_into(
                builder: &mut ::frieze::__private::frieze_usecase::SchemasBuilder,
            ) {
                #register_body
            }
        }
        // Marker impl: a struct-derived `Schema` is a struct schema.
        // Enums never receive this impl, so a `oneOf` newtype variant
        // referencing an enum-derived type is rejected at compile time
        // by the bound check emitted in `expand_enum`.
        impl #impl_generics ::frieze::__private::frieze_usecase::IsStructSchema for #ident #ty_generics #where_clause {}
        // Marker impl: a struct-derived `Schema` is registrable on a
        // `Schemas` collection. Primitive scalars never receive this
        // impl, so `Schemas::add::<i64>()` is rejected at compile time.
        impl #impl_generics ::frieze::__private::frieze_usecase::IsRegistrable for #ident #ty_generics #where_clause {}

        #inventory_submit
    };
    Ok(expanded)
}

/// Reject lifetime parameters and const generics with a curated message.
/// Type parameters fall through unchanged for downstream handling.
fn reject_unsupported_generic_params(
    generics: &syn::Generics,
    ident: &Ident,
) -> Result<(), syn::Error> {
    for param in &generics.params {
        match param {
            GenericParam::Type(_) => {}
            GenericParam::Lifetime(lifetime) => {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    format!(
                        "frieze: lifetime parameters are not supported in \
                         #[derive(Schema)] (struct `{ident}`). frieze schemas \
                         describe owned data layouts; remove the lifetime \
                         parameter or wrap the borrowed data in an owned type."
                    ),
                ));
            }
            GenericParam::Const(const_param) => {
                return Err(syn::Error::new_spanned(
                    const_param,
                    format!(
                        "frieze: const generics are not supported in \
                         #[derive(Schema)] (struct `{ident}`)."
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Build the token stream that computes the schema name at runtime.
///
/// - Non-generic structs compose `compose_schema_name(module_path!(), "<Base>")`.
/// - Generic structs first build the suffix-composed base
///   (`format!("{}_..._<Base>", T1::name(), ...)`) and then pass that
///   through `compose_schema_name`.
///
/// `compose_schema_name` consults the `Namespace` side channel
/// populated by `#[frieze(namespace)]` — when no namespace declarations
/// reach this binary the call is the identity, so the emitted name
/// keeps the pre-PR-1.5 value byte-for-byte.
fn composed_name_body(generics: &syn::Generics, base_name: &str) -> TokenStream {
    let type_param_idents: Vec<&Ident> = generics.type_params().map(|tp| &tp.ident).collect();
    if type_param_idents.is_empty() {
        return quote! {
            ::frieze::__private::compose_schema_name(
                ::core::module_path!(),
                #base_name,
            )
        };
    }
    // Suffix form per the design: `<Arg1>_<Arg2>_..._<Base>`. The
    // format string is `{}_` repeated N times followed by the literal
    // base name; the arguments are `<T_i as Schema>::name()` for each
    // type parameter in declaration order. The result is then wrapped
    // in `compose_schema_name` so namespace prefixes from the derive
    // site's `module_path!()` participate identically to the
    // non-generic path.
    let mut format_str = String::new();
    for _ in 0..type_param_idents.len() {
        format_str.push_str("{}_");
    }
    format_str.push_str(base_name);
    let args = type_param_idents.iter().map(|t| {
        quote! { <#t as ::frieze::__private::frieze_usecase::Schema>::name() }
    });
    quote! {
        ::frieze::__private::compose_schema_name(
            ::core::module_path!(),
            &::std::format!(#format_str, #(#args),*),
        )
    }
}
