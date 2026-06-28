//! Proc-macro derives for frieze.
//!
//! `#[derive(Schema)]` generates an implementation of the `frieze::Schema`
//! trait. Two top-level shapes are supported:
//!
//! - **Named struct** — every field type must come from the small fixed
//!   scalar set, optionally composed with `Vec<T>`, `Option<T>`, and/or
//!   `Maybe<T>`, or be itself a `Schema`-deriving type (rendered as
//!   `$ref`). The presence/nullability mapping is documented in the
//!   table below.
//! - **Unit-variant enum** — every variant must be a unit variant. The
//!   derive emits a `type: string, enum: [...]` schema whose values are
//!   each variant's wire name, computed from the variant identifier
//!   with any container-level `#[serde(rename_all = "...")]` applied
//!   first and any variant-level `#[serde(rename = "literal")]`
//!   overriding it. Tuple variants, struct variants, and empty enums
//!   are compile errors.
//!
//! Any other shape produces a compile error.
//!
//! # Rust shape → OAS combination
//!
//! The struct mapping is driven by syntactic type recognition plus a
//! small fixed set of serde attributes the macro reads: `rename`,
//! `rename_all`, `default`, and `skip_serializing_if`. Any other
//! `#[serde(...)]` entry that frieze cannot faithfully encode into a
//! single OAS schema (`alias`, `flatten`, `tag`, `content`, `untagged`,
//! `transparent`, `rename_all_fields`, `with` / `serialize_with` /
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

use std::collections::BTreeMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DeriveInput, Field, Fields, GenericArgument,
    Ident, PathArguments, Type, Variant,
};

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

fn expand_struct(ast: &DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
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

/// Expand `#[derive(Schema)]` on a unit-variant enum into an `impl Schema`
/// whose `schema()` returns a `StringEnum` variant.
///
/// Rejects empty enums and non-unit variants. The container-level
/// `#[serde(rename_all = "...")]` is applied to each variant's wire
/// name, and a variant-level `#[serde(rename = "literal")]` overrides
/// the container rule. Wire-name collisions across variants (whether
/// from literal `rename` or from a `rename_all` collapse) raise a
/// compile error.
fn expand_enum(ast: &DeriveInput, data: &DataEnum) -> Result<proc_macro2::TokenStream, syn::Error> {
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

/// Serde's container-level `rename_all` modes. Variant-name remapping is
/// implemented in [`RenameAll::apply`] so the macro reproduces the same
/// output serde produces at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameAll {
    None,
    Lowercase,
    Uppercase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

/// Site at which a `rename_all` rule is applied. Serde uses different
/// rules for struct fields (canonical input shape: `snake_case`) and
/// enum variants (canonical input shape: `PascalCase`), so the same
/// mode produces different output depending on the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameTarget {
    Field,
    Variant,
}

impl RenameAll {
    /// Apply the rule to an identifier. Mirrors serde's
    /// `apply_to_field` / `apply_to_variant` so the generated names
    /// match what serde will produce on the wire — picking the right
    /// branch from `target`.
    fn apply(self, name: &str, target: RenameTarget) -> String {
        match target {
            RenameTarget::Field => self.apply_to_field(name),
            RenameTarget::Variant => self.apply_to_variant(name),
        }
    }

    /// Variant rule: canonical input is `PascalCase`. Mirrors serde's
    /// `RenameRule::apply_to_variant`.
    fn apply_to_variant(self, variant: &str) -> String {
        match self {
            RenameAll::None | RenameAll::PascalCase => variant.to_owned(),
            RenameAll::Lowercase => variant.to_ascii_lowercase(),
            RenameAll::Uppercase => variant.to_ascii_uppercase(),
            RenameAll::CamelCase => {
                let mut chars = variant.chars();
                match chars.next() {
                    Some(first) => {
                        let mut out = String::with_capacity(variant.len());
                        out.extend(first.to_lowercase());
                        out.extend(chars);
                        out
                    }
                    None => String::new(),
                }
            }
            RenameAll::SnakeCase => pascal_or_camel_to_snake(variant),
            RenameAll::ScreamingSnakeCase => pascal_or_camel_to_snake(variant).to_ascii_uppercase(),
            RenameAll::KebabCase => pascal_or_camel_to_snake(variant).replace('_', "-"),
            RenameAll::ScreamingKebabCase => pascal_or_camel_to_snake(variant)
                .to_ascii_uppercase()
                .replace('_', "-"),
        }
    }

    /// Field rule: canonical input is `snake_case`. Mirrors serde's
    /// `RenameRule::apply_to_field`. The PascalCase / CamelCase
    /// branches collapse `_<letter>` boundaries by capitalising the
    /// following letter.
    fn apply_to_field(self, field: &str) -> String {
        match self {
            RenameAll::None | RenameAll::Lowercase | RenameAll::SnakeCase => field.to_owned(),
            RenameAll::Uppercase | RenameAll::ScreamingSnakeCase => field.to_ascii_uppercase(),
            RenameAll::PascalCase => snake_to_pascal(field),
            RenameAll::CamelCase => {
                let pascal = snake_to_pascal(field);
                let mut chars = pascal.chars();
                match chars.next() {
                    Some(first) => {
                        let mut out = String::with_capacity(pascal.len());
                        out.extend(first.to_lowercase());
                        out.extend(chars);
                        out
                    }
                    None => String::new(),
                }
            }
            RenameAll::KebabCase => field.replace('_', "-"),
            RenameAll::ScreamingKebabCase => field.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

/// PascalCase / camelCase → snake_case using serde's variant rule:
/// insert `_` before every uppercase letter (except at index 0), then
/// lowercase everything.
fn pascal_or_camel_to_snake(variant: &str) -> String {
    let mut out = String::with_capacity(variant.len() + 4);
    for (i, ch) in variant.char_indices() {
        if i > 0 && ch.is_ascii_uppercase() {
            out.push('_');
        }
        for lower in ch.to_lowercase() {
            out.push(lower);
        }
    }
    out
}

/// snake_case → PascalCase using serde's field rule: capitalise the
/// first character and every character following an `_`, dropping the
/// `_` separators.
fn snake_to_pascal(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut capitalise_next = true;
    for ch in field.chars() {
        if ch == '_' {
            capitalise_next = true;
        } else if capitalise_next {
            for upper in ch.to_uppercase() {
                out.push(upper);
            }
            capitalise_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Translate the `rename_all` literal captured by [`scan_serde_attrs`]
/// into a [`RenameAll`] mode. Returns `RenameAll::None` when no
/// `rename_all` attribute is present, and a compile error when the
/// literal isn't one of the eight recognised modes — the error points
/// at the literal so the highlight lands on what the user typed.
fn rename_all_from_scan(scan: &SerdeScan) -> Result<RenameAll, syn::Error> {
    match &scan.rename_all {
        Some((value, span)) => rename_all_from_str(value, *span),
        None => Ok(RenameAll::None),
    }
}

fn rename_all_from_str(value: &str, span: proc_macro2::Span) -> Result<RenameAll, syn::Error> {
    let mode = match value {
        "lowercase" => RenameAll::Lowercase,
        "UPPERCASE" => RenameAll::Uppercase,
        "PascalCase" => RenameAll::PascalCase,
        "camelCase" => RenameAll::CamelCase,
        "snake_case" => RenameAll::SnakeCase,
        "SCREAMING_SNAKE_CASE" => RenameAll::ScreamingSnakeCase,
        "kebab-case" => RenameAll::KebabCase,
        "SCREAMING-KEBAB-CASE" => RenameAll::ScreamingKebabCase,
        _ => {
            return Err(syn::Error::new(
                span,
                format!(
                    "frieze: unsupported `rename_all` value `{value}`; supported values are \
                     lowercase, UPPERCASE, PascalCase, camelCase, snake_case, \
                     SCREAMING_SNAKE_CASE, kebab-case, SCREAMING-KEBAB-CASE."
                ),
            ));
        }
    };
    Ok(mode)
}

fn named_fields(
    ast: &DeriveInput,
) -> Result<&syn::punctuated::Punctuated<Field, syn::Token![,]>, syn::Error> {
    let data_struct = match &ast.data {
        Data::Struct(s) => s,
        Data::Enum(_) | Data::Union(_) => unreachable!("dispatched in `expand`"),
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

/// Decision entry point: given a struct field, return the
/// `PropertyType` expression and the `Presence` expression that should
/// be passed to `Property::new` in the generated code.
///
/// Recognises (in order): `Maybe<T>`, `Option<T>` (with serde
/// `skip_serializing_if` attribute discrimination), `Vec<T>`, scalar
/// types. See the crate-level docs for the full table.
///
/// The serde attribute scan is performed by the caller (in
/// [`expand_struct`]) so the wire-name calculation and the DEFER
/// rejection share a single attribute walk.
fn parse_field(
    field: &Field,
    scan: &SerdeScan,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), syn::Error> {
    let ty = &field.ty;

    if let Some(inner) = unwrap_maybe(ty) {
        // Block doubly-defined / nested cases at the Maybe layer.
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: Maybe<Option<T>> is ambiguous; nullability is doubly defined. Use Maybe<T> alone.",
            ));
        }
        if unwrap_maybe(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Maybe is not supported.",
            ));
        }
        validate_maybe_serde_attrs(field, scan)?;
        let element = inner_to_property_type_expr(inner, ty, "Maybe<...>")?;
        Ok((nullable_property_type_expr(element), presence_optional()))
    } else if let Some(inner) = unwrap_option(ty) {
        if unwrap_option(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: nested Option is not supported.",
            ));
        }
        if unwrap_maybe(inner).is_some() {
            return Err(syn::Error::new_spanned(
                ty,
                "frieze: Option<Maybe<T>> is ambiguous; presence is doubly defined. Use Maybe<T> alone.",
            ));
        }
        // Option<Vec<T>> and Option<Vec<Option<T>>> are both rendered as
        // "outer nullable array"; the items' own nullability is
        // independent.
        if let Some(vec_inner) = unwrap_vec(inner) {
            let element = vec_element_property_type_expr(ty, vec_inner)?;
            let array = array_property_type_expr(element);
            return Ok((nullable_property_type_expr(array), presence_required()));
        }
        // Scalar `T` inside Option — branch ② / ③.
        let scalar = scalar_property_type_expr(inner).map_err(|_| {
            syn::Error::new_spanned(ty, unsupported_inside_message(inner, "Option<...>"))
        })?;
        if is_option_skip_predicate(scan) {
            // Branch ③: optional, non-nullable.
            Ok((scalar, presence_optional()))
        } else {
            // Branch ② (serde default): required, nullable.
            Ok((nullable_property_type_expr(scalar), presence_required()))
        }
    } else if let Some(vec_inner) = unwrap_vec(ty) {
        let element = vec_element_property_type_expr(ty, vec_inner)?;
        Ok((array_property_type_expr(element), presence_required()))
    } else {
        // Pass the error through verbatim so the dedicated "qualified
        // paths" / "generic type parameters" messages from
        // `reference_property_type_expr` reach the user. The generic
        // fallback message already lives inside that helper.
        let pt = scalar_property_type_expr(ty)?;
        Ok((pt, presence_required()))
    }
}

/// Builds the items expression for `Vec<inner>`, rejecting compositions
/// not allowed inside `Vec` (nested `Vec`, `Maybe`). Allows `Option<T>`
/// inside `Vec` (rendered as `items: { nullable }`).
fn vec_element_property_type_expr(
    outer: &Type,
    vec_inner: &Type,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    if unwrap_vec(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: nested Vec is not supported.",
        ));
    }
    if unwrap_maybe(vec_inner).is_some() {
        return Err(syn::Error::new_spanned(
            outer,
            "frieze: Vec<Maybe<T>> is not allowed; array elements are always present on the wire. Use Vec<Option<T>> if elements may be null.",
        ));
    }
    if let Some(opt_inner) = unwrap_option(vec_inner) {
        // Recursive Option<...> inside Vec — block the same ambiguities as
        // at the top level.
        if unwrap_option(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: nested Option is not supported.",
            ));
        }
        if unwrap_maybe(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: Option<Maybe<T>> is ambiguous; presence is doubly defined.",
            ));
        }
        if unwrap_vec(opt_inner).is_some() {
            return Err(syn::Error::new_spanned(
                outer,
                "frieze: nested Vec is not supported.",
            ));
        }
        let scalar = scalar_property_type_expr(opt_inner).map_err(|_| {
            syn::Error::new_spanned(
                outer,
                unsupported_inside_message(opt_inner, "Vec<Option<...>>"),
            )
        })?;
        return Ok(nullable_property_type_expr(scalar));
    }
    scalar_property_type_expr(vec_inner).map_err(|_| {
        syn::Error::new_spanned(outer, unsupported_inside_message(vec_inner, "Vec<...>"))
    })
}

/// Builds the property-type expression for the inner type of `Maybe<inner>`.
/// `Maybe` may wrap a scalar `T` or a `Vec<T>` / `Vec<Option<T>>`.
fn inner_to_property_type_expr(
    inner: &Type,
    outer_for_span: &Type,
    container: &str,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    if let Some(vec_inner) = unwrap_vec(inner) {
        let element = vec_element_property_type_expr(outer_for_span, vec_inner)?;
        return Ok(array_property_type_expr(element));
    }
    scalar_property_type_expr(inner).map_err(|_| {
        syn::Error::new_spanned(outer_for_span, unsupported_inside_message(inner, container))
    })
}

/// Validates that a `Maybe<T>` field carries both serde attributes
/// required for the documented optional-and-nullable round-trip:
///
/// - `#[serde(default)]` (bare form), so a missing key deserialises to
///   `Maybe::Missing` via `Maybe::default()`.
/// - `#[serde(skip_serializing_if = "Maybe::is_missing")]`, so a
///   `Maybe::Missing` value is omitted from the serialised output rather
///   than emitted as `null` (which would collide with `Maybe::Null`).
///
/// Without these, missing / null / present collapse to two states on the
/// wire — a silent runtime breakage. The check enforces them at compile
/// time so users get a clear, actionable diagnostic.
///
/// `default = "..."` with a custom path does not satisfy the pair:
/// serde must call `Maybe::default()` (which yields `Maybe::Missing`); a
/// custom default would defeat the three-state mapping. The
/// `skip_serializing_if` value is matched by exact string against
/// `"Maybe::is_missing"`.
///
/// The two attributes themselves are extracted by [`scan_serde_attrs`];
/// this validator only inspects the resulting [`SerdeScan`] so a single
/// pass covers both DEFER rejection and the `Maybe<T>` gate.
fn validate_maybe_serde_attrs(field: &Field, scan: &SerdeScan) -> Result<(), syn::Error> {
    if has_maybe_attribute_pair(scan) {
        return Ok(());
    }
    let field_name = field
        .ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_default();
    let msg = if field_name.is_empty() {
        "frieze: `Maybe<T>` field requires both `#[serde(default)]` and `#[serde(skip_serializing_if = \"Maybe::is_missing\")]`. Add: #[serde(default, skip_serializing_if = \"Maybe::is_missing\")]".to_string()
    } else {
        format!(
            "frieze: `Maybe<T>` field `{field_name}` requires both `#[serde(default)]` and `#[serde(skip_serializing_if = \"Maybe::is_missing\")]`. Add: #[serde(default, skip_serializing_if = \"Maybe::is_missing\")]"
        )
    };
    Err(syn::Error::new_spanned(&field.ty, msg))
}

/// Maps a Rust scalar field type to the matching
/// `frieze_model::PropertyType`, emitted as a path that resolves through
/// the facade re-export.
///
/// Falls through to [`reference_property_type_expr`] when the type is a
/// single-segment, unparametrised identifier that isn't a known scalar —
/// such an identifier is treated as a nested struct reference and the
/// expansion emits a `PropertyType::Reference` whose name comes from
/// `<T as frieze::Schema>::name()`. The `Schema` bound itself is enforced
/// by rustc (the user sees the trait-bound diagnostic if the type doesn't
/// implement `Schema`).
///
/// Anything else — qualified paths, generic arguments, primitive types
/// not in the supported set — produces a compile error.
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
        _ => reference_property_type_expr(ty),
    }
}

/// Treats a single-segment, unparametrised identifier as a reference to
/// another `Schema`-implementing type, and emits the
/// `PropertyType::Reference` constructor call.
///
/// Rejects:
///
/// - qualified paths (`mymod::User`) — the macro can't reliably resolve
///   them, so we require the user to bring the type into scope.
/// - generic arguments (`Foo<u32>`) — generics over user schemas are
///   deferred to Phase 1 #11.
/// - any other shape (references, tuples, etc.) — falls back to the
///   generic "unsupported field type" error.
///
/// The Schema bound is enforced naturally by rustc when the generated
/// `<#ident as ::frieze::__private::frieze_usecase::Schema>::name()` call
/// fails to compile.
fn reference_property_type_expr(ty: &Type) -> Result<proc_macro2::TokenStream, syn::Error> {
    let path = match ty {
        Type::Path(p) if p.qself.is_none() => &p.path,
        _ => {
            return Err(syn::Error::new_spanned(
                ty,
                unsupported_message(&type_to_display(ty)),
            ));
        }
    };
    if path.segments.len() > 1 {
        return Err(syn::Error::new_spanned(
            ty,
            "frieze: qualified paths in field type are not supported. \
             Use a `use` statement to bring the type into scope.",
        ));
    }
    let segment = path
        .segments
        .first()
        .expect("path with >=1 segments has a first segment");
    if !matches!(segment.arguments, PathArguments::None) {
        return Err(syn::Error::new_spanned(
            ty,
            "frieze: generic type parameters in field type are not supported.",
        ));
    }
    let ident = &segment.ident;
    Ok(quote! {
        ::frieze::__private::frieze_model::PropertyType::Reference(
            ::frieze::__private::frieze_model::SchemaName::new(
                <#ident as ::frieze::__private::frieze_usecase::Schema>::name()
            )
            .expect("frieze: referenced schema name violates the OAS component-name pattern")
        )
    })
}

/// Wraps a `PropertyType` expression in `PropertyType::Array(Box::new(...))`.
fn array_property_type_expr(element: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        ::frieze::__private::frieze_model::PropertyType::Array(
            ::std::boxed::Box::new(#element)
        )
    }
}

/// Wraps a `PropertyType` expression in `PropertyType::Nullable(Box::new(...))`.
fn nullable_property_type_expr(inner: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        ::frieze::__private::frieze_model::PropertyType::Nullable(
            ::std::boxed::Box::new(#inner)
        )
    }
}

fn presence_required() -> proc_macro2::TokenStream {
    quote! { ::frieze::__private::frieze_model::Presence::Required }
}

fn presence_optional() -> proc_macro2::TokenStream {
    quote! { ::frieze::__private::frieze_model::Presence::Optional }
}

/// Standard "unsupported field type" error message, listing the Phase 1
/// supported shapes.
fn unsupported_message(rendered: &str) -> String {
    format!(
        "frieze: unsupported field type `{rendered}`; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Vec<Option<T>>, Option<T>, Option<Vec<T>>, Maybe<T> (for any supported scalar T). Future PRs will add more."
    )
}

/// Variant of [`unsupported_message`] that names the wrapper containing
/// the offending inner type (e.g. `Option<...>` / `Vec<...>` / `Maybe<...>`).
fn unsupported_inside_message(inner: &Type, container: &str) -> String {
    format!(
        "frieze: unsupported field type `{}` inside {container}; only the following are supported in Phase 1: i32, i64, u32, u64, f32, f64, bool, String, Vec<T>, Vec<Option<T>>, Option<T>, Option<Vec<T>>, Maybe<T> (for any supported scalar T). Future PRs will add more.",
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

/// If `ty` syntactically names `Maybe<T>` (via `Maybe`, `frieze::Maybe`,
/// or `frieze_model::Maybe`), returns `T`.
///
/// Plain `Maybe` is the form users get from `use frieze::Maybe;` (the
/// facade re-export); the longer paths cover users who reach into the
/// underlying crates directly.
fn unwrap_maybe(ty: &Type) -> Option<&Type> {
    unwrap_single_generic(
        ty,
        &[&["Maybe"], &["frieze", "Maybe"], &["frieze_model", "Maybe"]],
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

/// Collects every `#[doc = "..."]` attribute on `attrs` and stitches them
/// into a single description string.
///
/// Each line follows the rustdoc convention:
///
/// - One leading space character is stripped if present (`/// foo`
///   becomes `foo`); writing `///foo` with no space leaves the line
///   unchanged.
/// - Trailing whitespace on every line is trimmed.
///
/// Lines are joined with `\n`, and the final result has its trailing
/// whitespace and blank lines trimmed away.
///
/// `#[doc(hidden)]` (the list form) is a visibility hint for rustdoc,
/// not a description source — it is ignored. Non-string-literal `doc`
/// payloads (e.g. `#[doc = const_str]`) are likewise ignored: they are
/// not what users typically write, and rejecting them here would block
/// patterns we don't need to support.
///
/// If every collected line is empty, the function returns `None` so the
/// downstream constructor's "empty container omitted" normalization
/// stays consistent with what the user wrote.
fn parse_doc_attrs(attrs: &[Attribute]) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        // `#[doc = "..."]` is a name-value attribute; `#[doc(hidden)]`
        // is a list attribute (`Meta::List`). Only the former carries a
        // description.
        let name_value = match &attr.meta {
            syn::Meta::NameValue(nv) => nv,
            syn::Meta::List(_) | syn::Meta::Path(_) => continue,
        };
        let lit = match &name_value.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(s) => s,
                _ => continue,
            },
            _ => continue,
        };
        let raw = lit.value();
        // A multi-line doc literal (e.g. from `#[doc = "a\nb"]`) is
        // expanded into multiple logical lines so the per-line
        // normalization applies uniformly.
        for line in raw.split('\n') {
            let stripped = line.strip_prefix(' ').unwrap_or(line);
            let trimmed_end = stripped.trim_end();
            lines.push(trimmed_end.to_string());
        }
    }
    // Trim trailing empty lines from the joined result.
    while matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return None;
    }
    let joined = lines.join("\n");
    let trimmed = joined.trim_end().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Builds the `Option<String>` token a `with_description(...)` call
/// expects: `None` if the description is absent, `Some(<literal>)`
/// otherwise. Emitting `None` rather than `Some("")` keeps the
/// generated code aligned with the model side's empty-container
/// normalization (the `with_description` call is then a no-op).
fn description_token(description: &Option<String>) -> proc_macro2::TokenStream {
    match description {
        Some(s) => quote! { ::std::option::Option::Some(::std::string::String::from(#s)) },
        None => quote! { ::std::option::Option::<::std::string::String>::None },
    }
}

/// Composes the enum-level description by appending a bullet list of
/// per-variant docs (using each variant's post-`rename_all` name) to
/// the enum-level doc. Mirrors the rules table in `design/descriptions.md`
/// section B5:
///
/// |              | enum-level doc present | enum-level doc absent |
/// |--------------|-----------------------|-----------------------|
/// | any variant has doc | `<enum doc>\n\n- v: ...` | `- v: ...` |
/// | no variant has doc  | `<enum doc>` (no bullets) | `None` |
///
/// Variants with no doc are omitted from the bullet list (a bare
/// `- name:` row is noise for readers); they still appear in the
/// `enum` array on the OAS side. Multi-line variant docs are
/// continued with a 2-space indent so each bullet stays visually
/// attached to its line.
fn compose_enum_description(
    enum_doc: Option<&str>,
    variants: &[(String, Option<String>)],
) -> Option<String> {
    let bullets: Vec<String> = variants
        .iter()
        .filter_map(|(name, doc)| doc.as_deref().map(|d| (name, d)))
        .map(|(name, doc)| {
            let mut lines = doc.split('\n');
            let first = lines.next().unwrap_or("");
            let mut bullet = format!("- {name}: {first}");
            for cont in lines {
                bullet.push('\n');
                bullet.push_str("  ");
                bullet.push_str(cont);
            }
            bullet
        })
        .collect();

    match (enum_doc, bullets.is_empty()) {
        (Some(doc), false) => Some(format!("{doc}\n\n{}", bullets.join("\n"))),
        (Some(doc), true) => Some(doc.to_string()),
        (None, false) => Some(bullets.join("\n")),
        (None, true) => None,
    }
}

/// Position context for [`scan_serde_attrs`]. Currently unused for
/// branching — every position rejects the same DEFER attribute set —
/// but kept so future changes can vary the diagnostic per site without
/// reshuffling call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerdePosition {
    StructContainer,
    StructField,
    EnumContainer,
    EnumVariant,
}

/// A single, deduplicated pass over the `#[serde(...)]` attributes on
/// one Rust item.
///
/// Walks every meta entry once and either:
///
/// - records a SUPPORT value into one of the fields, or
/// - rejects an unsupported (DEFER) form with a compile error, or
/// - silently skips an attribute frieze does not interpret (e.g.
///   `crate = "..."`) after consuming its value so the walker can
///   continue.
///
/// Returning a single struct rather than scattering one-purpose readers
/// across the macro keeps every diagnostic for a given site routed
/// through a single entry point, which matters most for DEFER rejects:
/// any code path that touches user attributes goes through this scan,
/// so an unsupported attribute cannot slip past one branch and be
/// silently honored by another.
#[derive(Debug, Default)]
struct SerdeScan {
    /// `#[serde(rename = "literal")]`, if present. Direction-split forms
    /// `#[serde(rename(serialize = ..., deserialize = ...))]` produce an
    /// error during scanning and therefore never reach this field.
    rename: Option<(String, proc_macro2::Span)>,
    /// `#[serde(rename_all = "literal")]`, if present. Same
    /// direction-split rejection as `rename`.
    rename_all: Option<(String, proc_macro2::Span)>,
    /// `true` when the field carries the bare `#[serde(default)]`.
    /// Custom-default forms like `#[serde(default = "path")]` are
    /// consumed but do **not** set this flag — the `Maybe<T>` gate
    /// requires the bare form.
    default_bare: bool,
    /// The literal string of the `skip_serializing_if = "..."`
    /// attribute, if present. The two values currently recognised by
    /// frieze are `"Option::is_none"` (switches an `Option<T>` field to
    /// optional / non-nullable) and `"Maybe::is_missing"` (part of the
    /// required `Maybe<T>` attribute pair). Other predicates are stored
    /// verbatim but have no effect on the generated schema.
    skip_serializing_if: Option<String>,
}

const DIRECTION_SPLIT_RENAME_MSG: &str = "frieze: `#[serde(rename(serialize = ..., deserialize = ...))]` produces different wire names for serialize and deserialize. A single OAS schema cannot represent both. Use a symmetric `#[serde(rename = \"...\")]` instead, or split the type into request- and response-shaped variants.";

const DIRECTION_SPLIT_RENAME_ALL_MSG: &str = "frieze: `#[serde(rename_all(serialize = ..., deserialize = ...))]` produces different wire names for serialize and deserialize. A single OAS schema cannot represent both. Use a symmetric `#[serde(rename_all = \"...\")]` instead, or split the type into request- and response-shaped variants.";

const EMPTY_RENAME_MSG: &str =
    "frieze: `#[serde(rename = \"\")]` is not supported. The wire name must be a non-empty string.";

/// Walk every `#[serde(...)]` attribute and classify each nested meta
/// entry into the [`SerdeScan`] record. DEFER attributes raise a compile
/// error here so a single scan covers both data extraction and the
/// unsupported-attribute check.
///
/// The `position` argument is currently informational — every Rust site
/// rejects the same DEFER set — but kept so future versions can tighten
/// the rules per site without rerouting calls.
fn scan_serde_attrs(
    attrs: &[Attribute],
    _position: SerdePosition,
) -> Result<SerdeScan, syn::Error> {
    let mut scan = SerdeScan::default();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let name = match meta.path.get_ident() {
                Some(i) => i.to_string(),
                None => {
                    // Qualified meta paths inside `#[serde(...)]` are not
                    // standard, but consume any value form so the walker
                    // can advance.
                    consume_meta_value(&meta)?;
                    return Ok(());
                }
            };
            match name.as_str() {
                "rename" => {
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(DIRECTION_SPLIT_RENAME_MSG));
                    }
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    if scan.rename.is_none() {
                        scan.rename = Some((lit.value(), lit.span()));
                    }
                }
                "rename_all" => {
                    if meta.input.peek(syn::token::Paren) {
                        return Err(meta.error(DIRECTION_SPLIT_RENAME_ALL_MSG));
                    }
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    if scan.rename_all.is_none() {
                        scan.rename_all = Some((lit.value(), lit.span()));
                    }
                }
                "default" => {
                    if meta.input.peek(syn::Token![=]) {
                        // `default = "path"`: consume and ignore. The
                        // bare form is the only one the `Maybe<T>` gate
                        // accepts, so a custom default is effectively
                        // not bare.
                        let _: syn::LitStr = meta.value()?.parse()?;
                    } else {
                        scan.default_bare = true;
                    }
                }
                "skip_serializing_if" => {
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    scan.skip_serializing_if = Some(lit.value());
                }
                "alias" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(alias)]` produces a deserialize-only acceptance list that a single OAS schema cannot encode."));
                }
                "flatten" => {
                    return Err(meta.error("frieze: `#[serde(flatten)]` on a field is not supported."));
                }
                "tag" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(tag)]` (internally-tagged enums) is not supported."));
                }
                "content" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(content)]` is not supported."));
                }
                "untagged" => {
                    return Err(meta.error("frieze: `#[serde(untagged)]` enums are not supported."));
                }
                "transparent" => {
                    return Err(meta.error("frieze: `#[serde(transparent)]` on a container is not supported."));
                }
                "other" => {
                    return Err(meta.error("frieze: `#[serde(other)]` on a variant is not supported."));
                }
                "rename_all_fields" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(rename_all_fields)]` is not supported."));
                }
                "skip" => {
                    return Err(meta.error("frieze: `#[serde(skip)]` is not supported."));
                }
                "skip_serializing" => {
                    return Err(meta.error("frieze: `#[serde(skip_serializing)]` is not supported."));
                }
                "skip_deserializing" => {
                    return Err(meta.error("frieze: `#[serde(skip_deserializing)]` is not supported."));
                }
                "with" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(with)]` is not supported."));
                }
                "serialize_with" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(serialize_with)]` is not supported."));
                }
                "deserialize_with" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(deserialize_with)]` is not supported."));
                }
                "from" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(from)]` is not supported."));
                }
                "try_from" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(try_from)]` is not supported."));
                }
                "into" => {
                    let _: syn::LitStr = meta.value()?.parse()?;
                    return Err(meta.error("frieze: `#[serde(into)]` is not supported."));
                }
                _ => {
                    // Unknown to frieze. Consume the value (if any) so
                    // the walker can move past this entry and silently
                    // skip — serde may grow more attributes that have
                    // no schema impact, and we don't want to break user
                    // code over them.
                    consume_meta_value(&meta)?;
                }
            }
            Ok(())
        })?;
    }
    Ok(scan)
}

/// Drain whatever follows a meta path so [`parse_nested_meta`] can
/// advance to the next comma-separated entry. Handles three shapes:
///
/// - `name = <expr>` — parse and discard the right-hand side.
/// - `name(<tokens>)` — parse and discard the parenthesised body.
/// - `name` alone — nothing to consume; return `Ok` immediately.
fn consume_meta_value(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<(), syn::Error> {
    if meta.input.peek(syn::Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _: proc_macro2::TokenStream = content.parse()?;
    }
    Ok(())
}

/// How a wire name was derived. Carried alongside the wire name so the
/// uniqueness check can report which input produced each conflicting
/// name.
#[derive(Debug, Clone)]
enum WireSource {
    /// The Rust identifier was used verbatim (no `rename`, no applicable
    /// `rename_all`).
    Identifier,
    /// An individual `#[serde(rename = "literal")]` produced the name.
    Individual,
    /// The container-level `#[serde(rename_all = "<mode>")]` produced
    /// the name; the mode string is kept so the diagnostic can quote it.
    RenameAll(&'static str),
}

impl WireSource {
    fn describe(&self) -> String {
        match self {
            WireSource::Identifier => "the Rust identifier".to_string(),
            WireSource::Individual => "`#[serde(rename = \"...\")]`".to_string(),
            WireSource::RenameAll(mode) => format!("`#[serde(rename_all = \"{mode}\")]`"),
        }
    }
}

/// Compute the wire name for one field or variant, applying the
/// precedence rule (individual `rename` > container `rename_all` > Rust
/// identifier).
///
/// Returns the wire name plus a [`WireSource`] tag for the uniqueness
/// check's diagnostic. Empty results — either an explicit `rename = ""`
/// or a `rename_all` rule that produces an empty string — are rejected
/// with [`EMPTY_RENAME_MSG`] anchored at the identifier's span.
fn wire_name(
    rust_ident: &Ident,
    individual: Option<(&str, proc_macro2::Span)>,
    container_rule: RenameAll,
    target: RenameTarget,
) -> Result<(String, WireSource), syn::Error> {
    if let Some((value, span)) = individual {
        if value.is_empty() {
            return Err(syn::Error::new(span, EMPTY_RENAME_MSG));
        }
        return Ok((value.to_string(), WireSource::Individual));
    }
    let ident_str = rust_ident.to_string();
    let converted = container_rule.apply(&ident_str, target);
    if converted.is_empty() {
        // In practice unreachable — Rust identifiers are never empty —
        // but kept defensive in case a future rule synthesises a name.
        return Err(syn::Error::new(rust_ident.span(), EMPTY_RENAME_MSG));
    }
    let source = match container_rule {
        RenameAll::None => WireSource::Identifier,
        other => WireSource::RenameAll(rename_all_label(other)),
    };
    Ok((converted, source))
}

/// Human-readable label for a `RenameAll` mode, used in collision
/// diagnostics so the message shows the exact attribute the user wrote.
fn rename_all_label(rule: RenameAll) -> &'static str {
    match rule {
        RenameAll::None => "",
        RenameAll::Lowercase => "lowercase",
        RenameAll::Uppercase => "UPPERCASE",
        RenameAll::PascalCase => "PascalCase",
        RenameAll::CamelCase => "camelCase",
        RenameAll::SnakeCase => "snake_case",
        RenameAll::ScreamingSnakeCase => "SCREAMING_SNAKE_CASE",
        RenameAll::KebabCase => "kebab-case",
        RenameAll::ScreamingKebabCase => "SCREAMING-KEBAB-CASE",
    }
}

/// Detect wire-name collisions across a struct's fields or an enum's
/// variants. The error message names both the second site (where the
/// span points) and the first site, including how each name was
/// produced, so the user can see at a glance whether the conflict comes
/// from a literal collision, a `rename_all` collapse, or both.
///
/// `kind` is the noun used in the message — `"field"` for struct fields,
/// `"variant"` for enum variants — so a single helper covers both
/// shapes.
fn check_unique_wire_names(
    entries: &[(Ident, String, WireSource)],
    kind: &str,
) -> Result<(), syn::Error> {
    let mut seen: BTreeMap<&str, (&Ident, &WireSource)> = BTreeMap::new();
    for (ident, wire, source) in entries {
        if let Some((prev_ident, prev_source)) = seen.get(wire.as_str()) {
            let msg = format!(
                "frieze: {kind} `{}` renames to {wire:?} via {}, which conflicts with {kind} `{}` (also renames to {wire:?} via {}). Each {kind} must have a unique wire name.",
                ident,
                source.describe(),
                prev_ident,
                prev_source.describe(),
            );
            return Err(syn::Error::new_spanned(ident, msg));
        }
        seen.insert(wire.as_str(), (ident, source));
    }
    Ok(())
}

/// Returns `true` when the scanned attributes carry
/// `#[serde(skip_serializing_if = "Option::is_none")]`. Replaces the
/// older one-purpose attribute walker; the value comes from the central
/// scan so DEFER attributes can't slip past on the same field.
fn is_option_skip_predicate(scan: &SerdeScan) -> bool {
    scan.skip_serializing_if.as_deref() == Some("Option::is_none")
}

/// Returns `true` when the field carries the `Maybe<T>` attribute pair
/// (`#[serde(default, skip_serializing_if = "Maybe::is_missing")]`).
/// Drives [`validate_maybe_serde_attrs`].
fn has_maybe_attribute_pair(scan: &SerdeScan) -> bool {
    scan.default_bare && scan.skip_serializing_if.as_deref() == Some("Maybe::is_missing")
}

#[cfg(test)]
mod doc_tests {
    use super::*;
    use syn::parse_quote;

    fn doc_attr(s: &str) -> Attribute {
        parse_quote!(#[doc = #s])
    }

    #[test]
    fn no_doc_attrs_returns_none() {
        let attrs: Vec<Attribute> = vec![];
        assert_eq!(parse_doc_attrs(&attrs), None);
    }

    #[test]
    fn strips_single_leading_space() {
        let attrs = vec![doc_attr(" hello")];
        assert_eq!(parse_doc_attrs(&attrs), Some("hello".to_string()));
    }

    #[test]
    fn preserves_extra_leading_spaces() {
        // Only one leading space is stripped — the second space stays.
        let attrs = vec![doc_attr("  hello")];
        assert_eq!(parse_doc_attrs(&attrs), Some(" hello".to_string()));
    }

    #[test]
    fn no_leading_space_is_allowed() {
        let attrs = vec![doc_attr("hello")];
        assert_eq!(parse_doc_attrs(&attrs), Some("hello".to_string()));
    }

    #[test]
    fn joins_multiple_attrs_with_newlines() {
        let attrs = vec![doc_attr(" first"), doc_attr(" second")];
        assert_eq!(parse_doc_attrs(&attrs), Some("first\nsecond".to_string()));
    }

    #[test]
    fn trims_trailing_whitespace_per_line() {
        let attrs = vec![doc_attr(" trailing   ")];
        assert_eq!(parse_doc_attrs(&attrs), Some("trailing".to_string()));
    }

    #[test]
    fn drops_trailing_empty_lines() {
        let attrs = vec![doc_attr(" line one"), doc_attr(""), doc_attr("")];
        assert_eq!(parse_doc_attrs(&attrs), Some("line one".to_string()));
    }

    #[test]
    fn ignores_doc_hidden_list_form() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[doc(hidden)])];
        assert_eq!(parse_doc_attrs(&attrs), None);
    }

    #[test]
    fn ignores_doc_alias_list_form_alongside_real_doc() {
        let real = doc_attr(" the real doc");
        let alias: Attribute = parse_quote!(#[doc(alias = "other")]);
        let attrs = vec![real, alias];
        assert_eq!(parse_doc_attrs(&attrs), Some("the real doc".to_string()));
    }

    #[test]
    fn all_whitespace_returns_none() {
        let attrs = vec![doc_attr("   "), doc_attr("\t")];
        assert_eq!(parse_doc_attrs(&attrs), None);
    }

    #[test]
    fn multi_line_string_literal_is_split() {
        // Equivalent to `#[doc = "line one\nline two"]`.
        let attrs = vec![doc_attr(" line one\n line two")];
        assert_eq!(
            parse_doc_attrs(&attrs),
            Some("line one\nline two".to_string())
        );
    }

    #[test]
    fn compose_enum_description_top_only() {
        let variants = vec![("Active".to_string(), None), ("Inactive".to_string(), None)];
        assert_eq!(
            compose_enum_description(Some("lifecycle"), &variants),
            Some("lifecycle".to_string())
        );
    }

    #[test]
    fn compose_enum_description_variant_only() {
        let variants = vec![
            ("Active".to_string(), Some("running".to_string())),
            ("Inactive".to_string(), None),
        ];
        assert_eq!(
            compose_enum_description(None, &variants),
            Some("- Active: running".to_string())
        );
    }

    #[test]
    fn compose_enum_description_top_and_variants() {
        let variants = vec![
            ("Active".to_string(), Some("running".to_string())),
            ("Inactive".to_string(), Some("stopped".to_string())),
        ];
        assert_eq!(
            compose_enum_description(Some("lifecycle"), &variants),
            Some("lifecycle\n\n- Active: running\n- Inactive: stopped".to_string())
        );
    }

    #[test]
    fn compose_enum_description_partial_variants_omits_missing_rows() {
        let variants = vec![
            ("Red".to_string(), Some("crimson".to_string())),
            ("Green".to_string(), None),
            ("Blue".to_string(), Some("deep blue".to_string())),
        ];
        assert_eq!(
            compose_enum_description(None, &variants),
            Some("- Red: crimson\n- Blue: deep blue".to_string())
        );
    }

    #[test]
    fn compose_enum_description_indents_multi_line_variant_doc() {
        let variants = vec![("Active".to_string(), Some("running\nright now".to_string()))];
        assert_eq!(
            compose_enum_description(None, &variants),
            Some("- Active: running\n  right now".to_string())
        );
    }

    #[test]
    fn compose_enum_description_none_when_neither_present() {
        let variants = vec![("Active".to_string(), None)];
        assert_eq!(compose_enum_description(None, &variants), None);
    }

    // --- wire-name calculation -------------------------------------

    fn ident(s: &str) -> Ident {
        syn::Ident::new(s, proc_macro2::Span::call_site())
    }

    #[test]
    fn wire_name_uses_individual_rename_over_container_rule() {
        let id = ident("user_id");
        let (wire, source) = wire_name(
            &id,
            Some(("external_id", id.span())),
            RenameAll::CamelCase,
            RenameTarget::Field,
        )
        .unwrap();
        assert_eq!(wire, "external_id");
        assert!(matches!(source, WireSource::Individual));
    }

    #[test]
    fn wire_name_falls_back_to_container_rule() {
        let id = ident("user_id");
        let (wire, source) =
            wire_name(&id, None, RenameAll::CamelCase, RenameTarget::Field).unwrap();
        assert_eq!(wire, "userId");
        assert!(matches!(source, WireSource::RenameAll("camelCase")));
    }

    #[test]
    fn wire_name_uses_identifier_when_no_rule_applies() {
        let id = ident("user_id");
        let (wire, source) = wire_name(&id, None, RenameAll::None, RenameTarget::Field).unwrap();
        assert_eq!(wire, "user_id");
        assert!(matches!(source, WireSource::Identifier));
    }

    #[test]
    fn wire_name_rejects_empty_individual_rename() {
        let id = ident("user_id");
        let err = wire_name(
            &id,
            Some(("", id.span())),
            RenameAll::None,
            RenameTarget::Field,
        )
        .unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }

    // --- rename_all field / variant divergence ---------------------

    #[test]
    fn rename_all_camel_case_collapses_snake_field_to_camel() {
        assert_eq!(
            RenameAll::CamelCase.apply("user_id", RenameTarget::Field),
            "userId"
        );
        assert_eq!(
            RenameAll::CamelCase.apply("display_name", RenameTarget::Field),
            "displayName"
        );
    }

    #[test]
    fn rename_all_camel_case_lowercases_variant_first_letter() {
        assert_eq!(
            RenameAll::CamelCase.apply("InactiveSince", RenameTarget::Variant),
            "inactiveSince"
        );
    }

    #[test]
    fn rename_all_snake_case_is_noop_for_already_snake_fields() {
        assert_eq!(
            RenameAll::SnakeCase.apply("user_id", RenameTarget::Field),
            "user_id"
        );
    }

    #[test]
    fn rename_all_snake_case_inserts_underscores_in_pascal_variants() {
        assert_eq!(
            RenameAll::SnakeCase.apply("InactiveSince", RenameTarget::Variant),
            "inactive_since"
        );
    }

    #[test]
    fn rename_all_pascal_case_collapses_snake_field_to_pascal() {
        assert_eq!(
            RenameAll::PascalCase.apply("user_id", RenameTarget::Field),
            "UserId"
        );
    }

    #[test]
    fn rename_all_kebab_case_replaces_underscores_in_fields() {
        assert_eq!(
            RenameAll::KebabCase.apply("user_id", RenameTarget::Field),
            "user-id"
        );
    }

    // --- uniqueness check -------------------------------------------

    #[test]
    fn check_unique_wire_names_passes_for_distinct_names() {
        let entries = vec![
            (
                ident("user_id"),
                "userId".to_string(),
                WireSource::RenameAll("camelCase"),
            ),
            (
                ident("display_name"),
                "displayName".to_string(),
                WireSource::RenameAll("camelCase"),
            ),
        ];
        check_unique_wire_names(&entries, "field").unwrap();
    }

    #[test]
    fn check_unique_wire_names_rejects_collision() {
        let entries = vec![
            (ident("user_id"), "id".to_string(), WireSource::Individual),
            (ident("id"), "id".to_string(), WireSource::Identifier),
        ];
        let err = check_unique_wire_names(&entries, "field").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("field `id`"));
        assert!(msg.contains("conflicts with field `user_id`"));
        assert!(msg.contains("unique wire name"));
    }
}
