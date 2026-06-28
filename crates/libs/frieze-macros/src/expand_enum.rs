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
//! - tag attribute on a unit-only enum (E-7) / data-carrying variant
//!   without tag (E-1) / unit variant mixed into a tagged enum
//!   (E-2a) — checked in [`expand_enum`] / [`expand_one_of`].
//!
//! Variant wire names follow the container `rename_all` / per-variant
//! `rename` precedence implemented in [`crate::rename::wire_name`], and
//! must be pairwise distinct (the same uniqueness check that guards
//! struct field wire names and unit-enum variant wire names).

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataEnum, DeriveInput, Fields, Ident, PathArguments, Type, Variant};

use crate::doc::{compose_enum_description, description_token, parse_doc_attrs};
use crate::rename::{
    check_unique_wire_names, rename_all_from_scan, wire_name, RenameAll, RenameTarget, WireSource,
};
use crate::serde_scan::{scan_serde_attrs, SerdePosition};
use crate::ty::{type_to_display, unwrap_maybe, unwrap_option, unwrap_vec};

/// Variant shape recognised by the derive.
///
/// `Newtype` carries the inner `Type` for downstream inspection (the tag
/// branch needs the inner's identifier to construct the
/// `IsStructSchema` bound check and the runtime `SchemaName` reference;
/// the no-tag branch never reaches this variant because
/// data-carrying-without-tag is rejected up-front in [`expand_enum`]).
/// The inner `Type` is boxed so the `Unit` and `Newtype` variants stay
/// near each other in size (clippy::large_enum_variant).
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
    let container_scan = scan_serde_attrs(&ast.attrs, SerdePosition::EnumContainer)?;
    let container_rule = rename_all_from_scan(&container_scan)?;

    // Classify every variant up-front. Struct variants and tuple
    // variants with multiple fields fail here regardless of tag mode
    // (E-3 / E-4).
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
            // E-7: unit-only enum should not carry an internal tag.
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
        expand_one_of(ident, &ast.attrs, tag, &classified, container_rule)
    } else {
        if any_data {
            // E-1: data-carrying variant without tag.
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
        expand_string_enum(ident, &ast.attrs, &classified, container_rule)
    }
}

/// Recognise the syntactic shape of one variant.
///
/// Rejects struct variants (`Foo { ... }` — E-3) and tuple variants with
/// `n != 1` fields (E-4) unconditionally, since neither shape is
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

    let schema_name = ident.to_string();
    let value_literals = values.iter().map(|v| {
        quote! { ::std::string::String::from(#v) }
    });
    let enum_doc = parse_doc_attrs(enum_attrs);
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

/// Expand an internally-tagged enum (every variant a
/// newtype-of-Schema-struct) into a `OneOf` schema.
///
/// Per-variant checks raise:
///
/// - E-2a if a unit variant is mixed into the tagged enum;
/// - E-2b if a newtype inner is a primitive, a known wrapper (`Vec`,
///   `Option`, `Maybe`), a qualified path, or a generic type;
/// - the bound check at the end fires E-2c (rustc surfaces the
///   `IsStructSchema` `on_unimplemented` message) when the inner is
///   another enum.
fn expand_one_of(
    ident: &Ident,
    enum_attrs: &[syn::Attribute],
    tag: &str,
    classified: &[(&Variant, VariantShape)],
    container_rule: RenameAll,
) -> Result<TokenStream, syn::Error> {
    // (wire name, inner type identifier, span at variant, optional per-variant doc)
    let mut inner_entries: Vec<(String, Ident, proc_macro2::Span, Option<String>)> =
        Vec::with_capacity(classified.len());
    let mut wire_entries: Vec<(Ident, String, WireSource)> = Vec::with_capacity(classified.len());

    for (variant, shape) in classified {
        let variant_ident = &variant.ident;
        let inner_ty = match shape {
            VariantShape::Unit => {
                // E-2a: unit variant in a tagged enum.
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
        let inner_ident = extract_newtype_inner_ident(variant_ident, inner_ty)?;

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
        inner_entries.push((wire.clone(), inner_ident, variant.span(), variant_doc));
        wire_entries.push((variant_ident.clone(), wire, source));
    }
    check_unique_wire_names(&wire_entries, "variant")?;

    let schema_name = ident.to_string();
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
        .map(|(wire, inner_ident, _, doc)| {
            let doc_expr = description_token(doc);
            quote! {
                ::frieze::__private::frieze_model::OneOfVariant::new(
                    #wire,
                    ::frieze::__private::frieze_model::SchemaName::new(
                        <#inner_ident as ::frieze::__private::frieze_usecase::Schema>::name()
                    )
                    .expect("frieze: referenced schema name violates the OAS component-name pattern"),
                )
                .with_description(#doc_expr)
            }
        })
        .collect();

    let struct_bound_checks: Vec<TokenStream> = inner_entries
        .iter()
        .map(|(_, inner_ident, variant_span, _)| {
            // `const _: () = { ... }` evaluates at compile time; the
            // inner `_assert` must be `const fn` so it can be called
            // from const context. The trait bound on `_assert` fires
            // E-2c (rustc surfaces the `on_unimplemented` message on
            // `IsStructSchema`) when `#inner_ident` is an enum-derived
            // type that has no `impl IsStructSchema`. Anchoring with
            // `quote_spanned!` makes the diagnostic point at the
            // user's variant rather than at synthesised macro code.
            quote_spanned! { *variant_span =>
                const _: () = {
                    const fn _frieze_assert_struct_schema<
                        T: ::frieze::__private::frieze_usecase::IsStructSchema,
                    >() {
                    }
                    _frieze_assert_struct_schema::<#inner_ident>();
                };
            }
        })
        .collect();

    let expanded = quote! {
        impl ::frieze::__private::frieze_usecase::Schema for #ident {
            fn name() -> &'static str {
                #schema_name
            }
            fn schema() -> ::frieze::__private::frieze_model::Schema {
                ::frieze::__private::frieze_model::Schema::new_one_of(
                    #schema_name,
                    #tag_lit,
                    ::std::vec![ #( #variant_constructor_exprs ),* ],
                )
                .expect("frieze: derived oneOf schema satisfies invariants by construction")
                .with_description(#composed_description_expr)
            }
        }

        #( #struct_bound_checks )*
    };
    Ok(expanded)
}

/// For a newtype variant `Variant(Inner)`, extract `Inner`'s identifier
/// (rejecting primitives, wrappers, qualified paths, generic types).
///
/// The trait-bound check emitted downstream further refines this: an
/// `Inner` that is enum-derived (and therefore does not implement
/// [`frieze_usecase::IsStructSchema`]) fails to compile with the
/// `on_unimplemented` diagnostic attached to that trait.
fn extract_newtype_inner_ident(variant_ident: &Ident, inner: &Type) -> Result<Ident, syn::Error> {
    // E-2b: known wrappers cannot be the inner of an internal-tagged
    // newtype variant.
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
    // E-2b: primitive scalars.
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
    // E-2b: anything that is not a single-segment, unparametrised path.
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
    let seg = path.segments.first().expect("len == 1");
    if !matches!(seg.arguments, PathArguments::None) {
        return Err(syn::Error::new_spanned(
            inner,
            format!(
                "frieze: newtype variant `{variant_ident}` wraps a generic \
                 type (`{}`); concrete user types only.",
                type_to_display(inner)
            ),
        ));
    }
    Ok(seg.ident.clone())
}
