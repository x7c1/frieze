//! Doc-comment extraction and rendering.
//!
//! Collects `#[doc = "..."]` attributes into a single description string,
//! converts an `Option<String>` description into the token expression a
//! `with_description(...)` call expects, and composes the per-enum
//! description from the enum-level doc plus per-variant docs.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Attribute;

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
pub(crate) fn parse_doc_attrs(attrs: &[Attribute]) -> Option<String> {
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
pub(crate) fn description_token(description: &Option<String>) -> TokenStream {
    match description {
        Some(s) => quote! { ::std::option::Option::Some(::std::string::String::from(#s)) },
        None => quote! { ::std::option::Option::<::std::string::String>::None },
    }
}

/// Composes the enum-level description by appending a bullet list of
/// per-variant docs (using each variant's post-`rename_all` name) to
/// the enum-level doc. Mirrors the rules table for description encoding:
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
pub(crate) fn compose_enum_description(
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

#[cfg(test)]
mod tests {
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
}
