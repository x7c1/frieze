//! Expansion for `#[derive(Schema)]` on an `enum`.
//!
//! Two top-level shapes are recognised:
//!
//! - **Unit-variant enum** (no `#[serde(tag = "...")]`). Every variant
//!   must be a unit variant; the derive emits a `StringEnum` schema.
//! - **Internally-tagged enum** (`#[serde(tag = "...")]`). Every variant
//!   must be a newtype wrapping a `Schema`-implementing struct; the
//!   derive emits a `OneOf` schema with a top-level
//!   `discriminator: {propertyName: <tag>}` block and per-arm
//!   `allOf: [{$ref: <inner>}, {synthetic tag-property object}]`. The
//!   `discriminator.mapping` key is deliberately omitted — see the
//!   rationale in `frieze-usecase::to_value::one_of_to_value`.
//!
//! Shape rejection is partitioned across:
//!
//! - struct variants and tuple variants with multiple fields →
//!   compile-time error from [`classify_variant`], independent of tag
//!   mode (a struct or multi-field tuple variant is never representable
//!   in OAS by the current frieze rules);
//! - newtype inner that is a primitive / known wrapper (`Vec`,
//!   `Option`, `Maybe`) / not a single-segment identifier →
//!   compile-time error from [`extract_newtype_inner_ident`] in the
//!   tag branch;
//! - newtype inner that is an enum-derived type → compile-time error
//!   from the per-variant `IsStructSchema` bound check (the marker
//!   trait is implemented only for struct-derived `Schema`s; the
//!   diagnostic message ships with the trait declaration in
//!   `frieze-usecase::schema`);
//! - tag attribute on a unit-only enum / data-carrying variant
//!   without tag / unit variant mixed into a tagged enum —
//!   checked in [`expand_enum`] / [`expand_one_of`].
//!
//! Variant wire names follow the container `rename_all` / per-variant
//! `rename` precedence implemented in [`crate::rename::wire_name`], and
//! must be pairwise distinct (the same uniqueness check that guards
//! struct field wire names and unit-enum variant wire names).
//!
//! # Generic enums
//!
//! When the input carries type parameters (`enum Either<T, E> { ... }`),
//! the derive propagates them onto the emitted `impl` blocks with a
//! synthesised `T: Schema` bound on every type parameter (alongside the
//! user's own `where` clause, which is preserved verbatim), and the
//! generated `name()` composes the schema name from each parameter's
//! name in the **suffix** form `<Arg1>_<Arg2>_..._<BaseName>` — same
//! rule as `expand_struct`. Non-generic enums keep emitting the plain
//! literal base name.
//!
//! Lifetime parameters and const generics are rejected at compile time
//! with the same wording struct derive uses.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataEnum, DeriveInput, Fields, GenericParam, Generics, Ident, Type, Variant};

use crate::doc::{compose_enum_description, description_token, parse_doc_attrs};
use crate::register::{inventory_submit_token, register_into_body};
use crate::rename::{
    check_unique_wire_names, rename_all_from_scan, wire_name, RenameAll, RenameTarget, WireSource,
};
use crate::serde_scan::{scan_serde_attrs, SerdePosition};
use crate::ty::{type_to_display, unwrap_maybe, unwrap_option, unwrap_vec};

/// Variant shape recognised by the derive.
///
/// `Newtype` carries the inner `Type` for downstream inspection (the tag
/// branch needs the inner `Type` to construct the `IsStructSchema`
/// bound check and the runtime `SchemaName` reference; the no-tag branch
/// never reaches this variant because data-carrying-without-tag is
/// rejected up-front in [`expand_enum`]). The inner `Type` is boxed so
/// the `Unit` and `Newtype` variants stay near each other in size
/// (clippy::large_enum_variant).
enum VariantShape {
    Unit,
    Newtype(Box<Type>),
}

/// Entry point: classify variants, decide between the string-enum and
/// `oneOf` expansion paths, and dispatch.
pub(crate) fn expand_enum(ast: &DeriveInput, data: &DataEnum) -> Result<TokenStream, syn::Error> {
    let ident = &ast.ident;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: enums with no variants cannot be represented as an OAS schema.",
        ));
    }
    // Reject lifetime / const-generic parameters before doing anything
    // else so we don't waste downstream work on a shape we won't emit.
    reject_unsupported_generic_params(&ast.generics, ident)?;

    let container_scan = scan_serde_attrs(&ast.attrs, SerdePosition::EnumContainer)?;
    let container_rule = rename_all_from_scan(&container_scan)?;

    // Classify every variant up-front. Struct variants and tuple
    // variants with multiple fields fail here regardless of tag mode.
    let mut classified: Vec<(&Variant, VariantShape)> = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        classified.push((variant, classify_variant(variant, ident)?));
    }

    let all_unit = classified
        .iter()
        .all(|(_, shape)| matches!(shape, VariantShape::Unit));
    let any_data = classified
        .iter()
        .any(|(_, shape)| !matches!(shape, VariantShape::Unit));

    let tag_value = container_scan.tag.as_ref().map(|(value, _)| value.as_str());

    if let Some(tag) = tag_value {
        if all_unit {
            // Unit-only enum carrying an explicit internal tag:
            // rejected because the string-enum path (no tag) emits a
            // cleaner `{type: string, enum: [...]}` shape, while a
            // tagged unit-only enum would serialise to anonymous
            // `{<tag>: "..."}` wrapper objects.
            return Err(syn::Error::new_spanned(
                ident,
                format!(
                    "frieze: enum `{ident}` is unit-only and does not need \
                     `#[serde(tag = \"{tag}\")]`. Remove the attribute to emit \
                     it as a string enum (`{{type: string, enum: [...]}}`). \
                     frieze reserves `#[serde(tag = \"...\")]` for enums whose \
                     variants carry data."
                ),
            ));
        }
        expand_one_of(
            ident,
            &ast.generics,
            &ast.attrs,
            tag,
            &classified,
            container_rule,
        )
    } else {
        if any_data {
            // A data-carrying variant requires an internal tag so the
            // wire shape carries a discriminator; without one, the
            // emitted schema cannot distinguish the variants.
            return Err(syn::Error::new_spanned(
                ident,
                format!(
                    "frieze: enum `{ident}` has data-carrying variants but no \
                     `#[serde(tag = \"...\")]` attribute. Add an internal tag \
                     to declare the discriminator property, e.g. \
                     `#[serde(tag = \"kind\")]`. Choose a tag name that does \
                     not conflict with any field of the inner structs \
                     (commonly `type`, `kind`, `label`, `event_type`)."
                ),
            ));
        }
        expand_string_enum(
            ident,
            &ast.generics,
            &ast.attrs,
            &classified,
            container_rule,
        )
    }
}

/// Recognise the syntactic shape of one variant.
///
/// Rejects struct variants (`Foo { ... }`) and tuple variants with
/// `n != 1` fields unconditionally, since neither shape is
/// representable as a OAS schema under the current frieze rules. Unit
/// and newtype variants pass through and are dispatched by the caller
/// according to the container's tag-mode.
fn classify_variant(variant: &Variant, enum_ident: &Ident) -> Result<VariantShape, syn::Error> {
    let variant_ident = &variant.ident;
    match &variant.fields {
        Fields::Unit => Ok(VariantShape::Unit),
        Fields::Unnamed(unnamed) => {
            let n = unnamed.unnamed.len();
            if n == 1 {
                let inner = unnamed.unnamed.first().expect("len == 1").ty.clone();
                Ok(VariantShape::Newtype(Box::new(inner)))
            } else {
                Err(syn::Error::new_spanned(
                    variant,
                    format!(
                        "frieze: tuple variants with multiple fields (variant \
                         `{variant_ident}` has {n}) are not supported. Define \
                         a named newtype struct: \
                         #[derive(Schema)] struct {variant_ident} {{ ... }}; \
                         enum {enum_ident} {{ {variant_ident}({variant_ident}), ... }}."
                    ),
                ))
            }
        }
        Fields::Named(_) => Err(syn::Error::new_spanned(
            variant,
            format!(
                "frieze: struct variants (variant `{variant_ident}` has named \
                 fields) are not supported. Define a named struct and use a \
                 newtype variant: \
                 #[derive(Schema)] struct {variant_ident}Data {{ ... }}; \
                 enum {enum_ident} {{ {variant_ident}({variant_ident}Data), ... }}."
            ),
        )),
    }
}

/// Expand an enum whose every variant is a unit variant into a
/// `StringEnum` schema. Mirrors the pre-`oneOf` derive behaviour
/// verbatim — the only change is that the `Fields::Unit` precondition is
/// now established by [`classify_variant`] in the caller.
fn expand_string_enum(
    ident: &Ident,
    generics: &Generics,
    enum_attrs: &[syn::Attribute],
    classified: &[(&Variant, VariantShape)],
    container_rule: RenameAll,
) -> Result<TokenStream, syn::Error> {
    let mut values: Vec<String> = Vec::with_capacity(classified.len());
    let mut variant_descriptions: Vec<(String, Option<String>)> =
        Vec::with_capacity(classified.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> = Vec::with_capacity(classified.len());

    for (variant, _shape) in classified {
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

    let base_name = ident.to_string();
    let value_literals = values.iter().map(|v| {
        quote! { ::std::string::String::from(#v) }
    });
    let enum_doc = parse_doc_attrs(enum_attrs);
    let composed_description = compose_enum_description(enum_doc.as_deref(), &variant_descriptions);
    let composed_description_expr = description_token(&composed_description);

    let mut impl_generics_storage = generics.clone();
    push_schema_bound(&mut impl_generics_storage);
    let (impl_generics, ty_generics, where_clause) = impl_generics_storage.split_for_impl();
    let name_body = composed_name_body(generics, &base_name);
    // Unit-only enums have no inner types to recurse into; the body
    // collapses to the idempotent guard plus a single `push_unique`.
    let register_body = register_into_body(&[]);
    // Non-generic enum: emit one `inventory::submit!` site. Generic
    // enums (`enum E<T> { ... }`) produce an empty stream — see
    // `inventory_submit_token` for the rationale.
    let inventory_submit = inventory_submit_token(ident, generics);

    let expanded = quote! {
        impl #impl_generics ::frieze::__private::frieze_usecase::Schema for #ident #ty_generics #where_clause {
            fn name() -> ::std::string::String {
                #name_body
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new_string_enum(
                    <Self as ::frieze::__private::frieze_usecase::Schema>::name(),
                    ::std::vec![ #( #value_literals ),* ],
                )
                .expect("frieze: derived enum schema satisfies invariants by construction")
                .with_description(#composed_description_expr)
            }
            fn register_into(
                builder: &mut ::frieze::__private::frieze_usecase::SchemasBuilder,
            ) {
                #register_body
            }
        }
        // Marker impl: an enum-derived `Schema` is registrable on a
        // `Schemas` collection.
        impl #impl_generics ::frieze::__private::frieze_usecase::IsRegistrable for #ident #ty_generics #where_clause {}

        #inventory_submit
    };
    Ok(expanded)
}

/// Expand an internally-tagged enum (every variant a
/// newtype-of-Schema-struct) into a `OneOf` schema.
///
/// Per-variant checks raise:
///
/// - an error if a unit variant is mixed into the tagged enum (the
///   tagged wire shape would be indistinguishable from an empty
///   struct variant);
/// - an error if a newtype inner is a primitive, a known wrapper
///   (`Vec`, `Option`, `Maybe`), or a qualified path — none of which
///   is a struct that implements `Schema`. A generic-argument inner
///   (`Container<i64>`) is accepted: the `IsStructSchema` bound check
///   below verifies that the concrete instantiation is a struct schema;
/// - the per-variant `IsStructSchema` bound check at the end fires
///   (rustc surfaces the `on_unimplemented` message on the trait)
///   when the inner is itself an enum-derived `Schema`.
fn expand_one_of(
    ident: &Ident,
    generics: &Generics,
    enum_attrs: &[syn::Attribute],
    tag: &str,
    classified: &[(&Variant, VariantShape)],
    container_rule: RenameAll,
) -> Result<TokenStream, syn::Error> {
    // (wire name, inner type, span at variant, optional per-variant doc)
    let mut inner_entries: Vec<(String, Type, proc_macro2::Span, Option<String>)> =
        Vec::with_capacity(classified.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> = Vec::with_capacity(classified.len());

    for (variant, shape) in classified {
        let variant_ident = &variant.ident;
        let inner_ty = match shape {
            VariantShape::Unit => {
                // Unit variant in a tagged enum: the wire shape would
                // be `{"<tag>": "<variant>"}`, indistinguishable from
                // an empty struct variant. Either split unit variants
                // into a separate string-enum, or give them a payload.
                return Err(syn::Error::new_spanned(
                    variant,
                    format!(
                        "frieze: enum `{ident}` has `#[serde(tag = \"{tag}\")]` \
                         but contains a unit variant `{variant_ident}`. \
                         internal-tagged enums require every variant to be a \
                         newtype wrapping a Schema-implementing struct; unit \
                         variants would serialize to `{{\"{tag}\": \"{variant_ident}\"}}`, \
                         indistinguishable on the wire from an empty struct \
                         variant. Either split unit variants into a separate \
                         enum (emitted as `{{type: string, enum: [...]}}`), or \
                         give the unit variant a payload as a named struct."
                    ),
                ));
            }
            VariantShape::Newtype(ty) => ty,
        };
        let inner_type = extract_newtype_inner_type(variant_ident, inner_ty)?;

        let variant_scan = scan_serde_attrs(&variant.attrs, SerdePosition::EnumVariant)?;
        let individual = variant_scan
            .rename
            .as_ref()
            .map(|(s, sp)| (s.as_str(), *sp));
        let (wire, source) = wire_name(
            variant_ident,
            individual,
            container_rule,
            RenameTarget::Variant,
        )?;
        let variant_doc = parse_doc_attrs(&variant.attrs);
        inner_entries.push((wire.clone(), inner_type, variant.span(), variant_doc));
        wire_entries.push((variant_ident.clone(), wire, source));
    }
    check_unique_wire_names(&wire_entries, "variant")?;

    let base_name = ident.to_string();
    let tag_lit = tag.to_string();
    let enum_doc = parse_doc_attrs(enum_attrs);
    let variant_descriptions: Vec<(String, Option<String>)> = inner_entries
        .iter()
        .map(|(wire, _, _, doc)| (wire.clone(), doc.clone()))
        .collect();
    let composed_description = compose_enum_description(enum_doc.as_deref(), &variant_descriptions);
    let composed_description_expr = description_token(&composed_description);

    let variant_constructor_exprs: Vec<TokenStream> = inner_entries
        .iter()
        .map(|(wire, inner_type, _, doc)| {
            let doc_expr = description_token(doc);
            quote! {
                ::frieze::__private::frieze_model::OneOfVariant::new(
                    #wire,
                    ::frieze::__private::frieze_model::SchemaName::new(
                        <#inner_type as ::frieze::__private::frieze_usecase::Schema>::name()
                    )
                    .expect("frieze: referenced schema name violates the OAS component-name pattern"),
                )
                .with_description(#doc_expr)
            }
        })
        .collect();

    let has_generics = !generics.params.is_empty();
    let struct_bound_checks: Vec<TokenStream> = inner_entries
        .iter()
        .map(|(_, inner_type, variant_span, _)| {
            // The `_assert` function carries the `IsStructSchema`
            // trait bound; calling it on `#inner_type` makes rustc
            // surface the `on_unimplemented` message attached to
            // `IsStructSchema` when the inner type is an enum-derived
            // `Schema` (or any other type without an `IsStructSchema`
            // impl). Anchoring with `quote_spanned!` makes the
            // diagnostic point at the user's variant rather than at
            // synthesised macro code.
            //
            // Non-generic enums emit the assertion at top level via
            // `const _: () = { ... }` so it fires eagerly. For generic
            // enums the inner type contains the enum's type parameters
            // (e.g. `Container<T>`), so the check has to live inside the
            // `impl` block — we use `inline const { ... }` inside the
            // `schema()` body so the assertion is part of the
            // monomorphisation step and the type parameters are in
            // scope. The bound check still runs at compile time,
            // per-instantiation.
            if has_generics {
                quote_spanned! { *variant_span =>
                    {
                        const fn _frieze_assert_struct_schema<
                            T: ::frieze::__private::frieze_usecase::IsStructSchema,
                        >() {
                        }
                        _frieze_assert_struct_schema::<#inner_type>();
                    };
                }
            } else {
                quote_spanned! { *variant_span =>
                    const _: () = {
                        const fn _frieze_assert_struct_schema<
                            T: ::frieze::__private::frieze_usecase::IsStructSchema,
                        >() {
                        }
                        _frieze_assert_struct_schema::<#inner_type>();
                    };
                }
            }
        })
        .collect();

    let mut impl_generics_storage = generics.clone();
    push_schema_bound(&mut impl_generics_storage);
    let (impl_generics, ty_generics, where_clause) = impl_generics_storage.split_for_impl();
    let name_body = composed_name_body(generics, &base_name);

    let (inner_bound_checks, outer_bound_checks): (TokenStream, TokenStream) = if has_generics {
        (quote! { #( #struct_bound_checks )* }, quote! {})
    } else {
        (quote! {}, quote! { #( #struct_bound_checks )* })
    };

    // Each `oneOf` variant carries an inner struct type; the derived
    // `register_into` recurses into every one so adding the enum root
    // pulls all variant-inner schemas in transitively.
    let variant_inner_types: Vec<&Type> = inner_entries.iter().map(|(_, ty, _, _)| ty).collect();
    let register_body = register_into_body(&variant_inner_types);
    // Non-generic oneOf enum: emit one `inventory::submit!` site.
    // Generic oneOf enums produce an empty stream.
    let inventory_submit = inventory_submit_token(ident, generics);

    let expanded = quote! {
        impl #impl_generics ::frieze::__private::frieze_usecase::Schema for #ident #ty_generics #where_clause {
            fn name() -> ::std::string::String {
                #name_body
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                // Per-variant `IsStructSchema` bound checks for the
                // generic case — see `struct_bound_checks` above.
                #inner_bound_checks
                ::frieze::__private::frieze_model::Schema::new_one_of(
                    <Self as ::frieze::__private::frieze_usecase::Schema>::name(),
                    #tag_lit,
                    ::std::vec![ #( #variant_constructor_exprs ),* ],
                )
                .expect("frieze: derived oneOf schema satisfies invariants by construction")
                .with_description(#composed_description_expr)
            }
            fn register_into(
                builder: &mut ::frieze::__private::frieze_usecase::SchemasBuilder,
            ) {
                #register_body
            }
        }
        // Marker impl: an enum-derived `Schema` is registrable on a
        // `Schemas` collection.
        impl #impl_generics ::frieze::__private::frieze_usecase::IsRegistrable for #ident #ty_generics #where_clause {}

        #outer_bound_checks
        #inventory_submit
    };
    Ok(expanded)
}

/// For a newtype variant `Variant(Inner)`, extract `Inner` as a
/// [`syn::Type`] for downstream use in the runtime `SchemaName`
/// reference and the compile-time `IsStructSchema` bound check.
///
/// Rejects primitives, known wrappers (`Option`, `Vec`, `Maybe`),
/// qualified paths, and non-path type forms. Generic-argument inners
/// (`Container<i64>`) are accepted: the downstream `IsStructSchema`
/// bound check verifies that the concrete instantiation is a struct
/// schema. An `Inner` whose `Schema` is enum-derived (and therefore
/// does not implement [`frieze_usecase::IsStructSchema`]) fails to
/// compile with the `on_unimplemented` diagnostic attached to that
/// trait.
fn extract_newtype_inner_type(variant_ident: &Ident, inner: &Type) -> Result<Type, syn::Error> {
    // Known wrappers (`Option`, `Vec`, `Maybe`) cannot be the inner
    // of an internal-tagged newtype variant — they are not structs
    // that implement `Schema`.
    if unwrap_option(inner).is_some()
        || unwrap_vec(inner).is_some()
        || unwrap_maybe(inner).is_some()
    {
        return Err(syn::Error::new_spanned(
            inner,
            format!(
                "frieze: newtype variant `{variant_ident}` wraps `{}`, which is \
                 not a struct. internal-tagged enums require every newtype \
                 variant to wrap a struct that implements `Schema` (i.e. has \
                 #[derive(Schema)]). Define a named struct: \
                 #[derive(Schema)] struct {variant_ident}Data {{ ... }}.",
                type_to_display(inner)
            ),
        ));
    }
    let rendered = type_to_display(inner);
    // Primitive scalars are not structs that implement `Schema`.
    if matches!(
        rendered.as_str(),
        "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "String"
    ) {
        return Err(syn::Error::new_spanned(
            inner,
            format!(
                "frieze: newtype variant `{variant_ident}` wraps a non-Schema \
                 type (`{rendered}`). internal-tagged enums require every \
                 newtype variant to wrap a struct that implements `Schema` \
                 (i.e. has #[derive(Schema)]). Define a named struct: \
                 #[derive(Schema)] struct {variant_ident}Data {{ value: {rendered} }}."
            ),
        ));
    }
    // Reject anything that is not a single-segment path: qualified
    // paths, references, tuples, etc. Generic arguments on the single
    // segment are accepted.
    let path = match inner {
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => {
            return Err(syn::Error::new_spanned(
                inner,
                format!(
                    "frieze: newtype variant `{variant_ident}` wraps an \
                     unsupported type form (`{}`). internal-tagged variants \
                     require the inner to be a single-identifier struct path.",
                    type_to_display(inner)
                ),
            ));
        }
    };
    if path.segments.len() != 1 {
        return Err(syn::Error::new_spanned(
            inner,
            format!(
                "frieze: newtype variant `{variant_ident}` wraps a qualified \
                 path (`{}`); use a `use` statement to bring the type into \
                 scope.",
                type_to_display(inner)
            ),
        ));
    }
    Ok(inner.clone())
}

/// Reject lifetime parameters and const generics with a curated message
/// (the same wording used for struct derive). Type parameters fall
/// through unchanged for downstream handling.
fn reject_unsupported_generic_params(generics: &Generics, ident: &Ident) -> Result<(), syn::Error> {
    for param in &generics.params {
        match param {
            GenericParam::Type(_) => {}
            GenericParam::Lifetime(lifetime) => {
                return Err(syn::Error::new_spanned(
                    lifetime,
                    format!(
                        "frieze: lifetime parameters are not supported in \
                         #[derive(Schema)] (enum `{ident}`). frieze schemas \
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
                         #[derive(Schema)] (enum `{ident}`)."
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Push the synthesised `T: Schema` bound onto every type parameter in
/// `generics`. The user's own `where` clause is preserved by the
/// downstream `split_for_impl()` call.
fn push_schema_bound(generics: &mut Generics) {
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(type_param) = param {
            let bound: syn::TypeParamBound =
                syn::parse_quote!(::frieze::__private::frieze_usecase::Schema);
            type_param.bounds.push(bound);
        }
    }
}

/// Build the token stream that computes the schema name at runtime.
///
/// - Non-generic enums return the literal base name as a `String`,
///   byte-identical to the pre-generic expansion (snapshot stability).
/// - Generic enums return `format!("{}_..._<Base>", T1::name(), ...)`
///   using the same suffix-form composition rule that `expand_struct`
///   uses for generic structs.
fn composed_name_body(generics: &Generics, base_name: &str) -> TokenStream {
    let type_param_idents: Vec<&Ident> = generics.type_params().map(|tp| &tp.ident).collect();
    if type_param_idents.is_empty() {
        return quote! { ::std::string::String::from(#base_name) };
    }
    let mut format_str = String::new();
    for _ in 0..type_param_idents.len() {
        format_str.push_str("{}_");
    }
    format_str.push_str(base_name);
    let args = type_param_idents.iter().map(|t| {
        quote! { <#t as ::frieze::__private::frieze_usecase::Schema>::name() }
    });
    quote! {
        ::std::format!(#format_str, #(#args),*)
    }
}
