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
//!   the variant names after applying any container-level
//!   `#[serde(rename_all = "...")]`. Tuple variants, struct variants,
//!   empty enums, and `#[serde(rename)]` on individual variants are
//!   compile errors.
//!
//! Any other shape produces a compile error.
//!
//! # Rust shape → OAS combination
//!
//! The struct mapping is driven by syntactic type recognition plus a
//! single serde attribute (`skip_serializing_if = "Option::is_none"`);
//! the macro does not interpret any other serde attribute on fields.
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
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DeriveInput, Field, Fields, GenericArgument,
    PathArguments, Type, Variant,
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
    let fields = named_fields(ast)?;
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
        let (property_type_expr, presence_expr) = parse_field(field)?;
        property_exprs.push(quote! {
            ::frieze::__private::frieze_model::Property::new(
                #field_name,
                #property_type_expr,
                #presence_expr,
            )
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
                ::frieze::__private::frieze_model::Schema::new_object(
                    #schema_name,
                    ::std::vec![ #( #property_exprs ),* ],
                )
                .expect("frieze: derived schema satisfies invariants by construction")
            }
        }
    };
    Ok(expanded)
}

/// Expand `#[derive(Schema)]` on a unit-variant enum into an `impl Schema`
/// whose `schema()` returns a `StringEnum` variant.
///
/// Rejects empty enums, non-unit variants, and variant-level
/// `#[serde(rename = "...")]`. The container-level
/// `#[serde(rename_all = "...")]` is read and applied to the variant
/// names emitted into the `enum` array; an unsupported `rename_all`
/// value is a compile error pointing at the attribute.
fn expand_enum(ast: &DeriveInput, data: &DataEnum) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = &ast.ident;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            ident,
            "frieze: enums with no variants cannot be represented as an OAS schema.",
        ));
    }
    let rename_all = parse_rename_all(&ast.attrs)?;
    let mut values: Vec<String> = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        validate_variant(variant)?;
        if let Some(attr) = find_variant_rename(&variant.attrs) {
            return Err(syn::Error::new_spanned(
                attr,
                "frieze: `#[serde(rename)]` on a variant is not supported.",
            ));
        }
        let original = variant.ident.to_string();
        values.push(rename_all.apply(&original));
    }

    let schema_name = ident.to_string();
    let value_literals = values.iter().map(|v| {
        quote! { ::std::string::String::from(#v) }
    });
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

/// Returns the first `#[serde(rename = "...")]` attribute on a variant
/// so the caller can point its compile error at the offending attribute.
fn find_variant_rename(attrs: &[Attribute]) -> Option<&Attribute> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") && meta.input.peek(syn::Token![=]) {
                let _ = meta.value().and_then(|v| v.parse::<syn::LitStr>());
                found = true;
            }
            Ok(())
        });
        if found {
            return Some(attr);
        }
    }
    None
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

impl RenameAll {
    /// Apply the rule to a variant identifier. Mirrors serde's
    /// `apply_to_variant` so generated enums match what serde will
    /// produce on the wire.
    fn apply(self, variant: &str) -> String {
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
            RenameAll::SnakeCase => to_snake(variant),
            RenameAll::ScreamingSnakeCase => to_snake(variant).to_ascii_uppercase(),
            RenameAll::KebabCase => to_snake(variant).replace('_', "-"),
            RenameAll::ScreamingKebabCase => {
                to_snake(variant).to_ascii_uppercase().replace('_', "-")
            }
        }
    }
}

/// PascalCase / camelCase → snake_case using serde's variant rule:
/// insert `_` before every uppercase letter (except at index 0), then
/// lowercase everything.
fn to_snake(variant: &str) -> String {
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

/// Parse the container-level `#[serde(rename_all = "...")]` if present.
/// Unsupported values surface as a compile error pointing at the
/// attribute, listing the recognised modes.
fn parse_rename_all(attrs: &[Attribute]) -> Result<RenameAll, syn::Error> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut found: Option<(String, proc_macro2::Span)> = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                if let Ok(value) = meta.value() {
                    if let Ok(lit) = value.parse::<syn::LitStr>() {
                        found = Some((lit.value(), lit.span()));
                    }
                }
            }
            Ok(())
        });
        if let Some((value, span)) = found {
            return rename_all_from_str(&value, span, attr);
        }
    }
    Ok(RenameAll::None)
}

fn rename_all_from_str(
    value: &str,
    span: proc_macro2::Span,
    attr: &Attribute,
) -> Result<RenameAll, syn::Error> {
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
            let _ = span; // anchor the error on the attribute for tooling
            return Err(syn::Error::new_spanned(
                attr,
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
fn parse_field(
    field: &Field,
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
        validate_maybe_serde_attrs(field)?;
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
        if has_skip_serializing_if_option_is_none(&field.attrs) {
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
/// `default = "..."` with a custom path is rejected: serde must call
/// `Maybe::default()` (which yields `Maybe::Missing`); a custom default
/// would defeat the three-state mapping. The `skip_serializing_if` value
/// is matched by exact string against `"Maybe::is_missing"`.
fn validate_maybe_serde_attrs(field: &Field) -> Result<(), syn::Error> {
    let mut has_default_bare = false;
    let mut has_skip_serializing_if_maybe = false;
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        // Best-effort parse: serde carries attribute forms we don't
        // recognise. Ignore parse failures and only look at the two
        // pieces we care about.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                // Bare `default` has no `=` value waiting; anything else
                // (e.g. `default = "..."`) is rejected — consume the
                // value to keep nested-meta parsing well-formed.
                if meta.input.peek(syn::Token![=]) {
                    let _ = meta.value().and_then(|v| v.parse::<syn::LitStr>());
                } else {
                    has_default_bare = true;
                }
            } else if meta.path.is_ident("skip_serializing_if") {
                if let Ok(value) = meta.value() {
                    if let Ok(lit) = value.parse::<syn::LitStr>() {
                        if lit.value() == "Maybe::is_missing" {
                            has_skip_serializing_if_maybe = true;
                        }
                    }
                }
            }
            Ok(())
        });
    }
    if has_default_bare && has_skip_serializing_if_maybe {
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

/// Returns `true` if any `#[serde(...)]` attribute on the field contains
/// `skip_serializing_if = "Option::is_none"`.
///
/// The check is strictly syntactic — the right-hand side must be the
/// literal string `"Option::is_none"`. Other helper names (custom
/// predicates) do not switch the property into branch ③.
fn has_skip_serializing_if_option_is_none(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut matched = false;
        // Best-effort parse: serde may carry attribute forms we don't
        // recognise (e.g. lone idents like `default`), and `parse_nested_meta`
        // returns `Ok` for those when the callback accepts them. We only
        // care about the one name-value pair we look for and ignore the
        // result either way.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip_serializing_if") {
                if let Ok(value) = meta.value() {
                    if let Ok(lit) = value.parse::<syn::LitStr>() {
                        if lit.value() == "Option::is_none" {
                            matched = true;
                        }
                    }
                }
            }
            Ok(())
        });
        if matched {
            return true;
        }
    }
    false
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
