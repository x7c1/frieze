//! Wire-name calculation: serde's `rename` / `rename_all` rules,
//! identifier-to-wire translation, and uniqueness checking.
//!
//! [`wire_name`] is the entry point used by both struct-field expansion
//! and enum-variant expansion; the precedence (individual `rename` >
//! container `rename_all` > Rust identifier) is implemented here so the
//! two call sites share one source of truth.

use std::collections::BTreeMap;

use syn::Ident;

use crate::serde_scan::SerdeScan;

const EMPTY_RENAME_MSG: &str =
    "frieze: `#[serde(rename = \"\")]` is not supported. The wire name must be a non-empty string.";

/// Serde's container-level `rename_all` modes. Variant-name remapping is
/// implemented in [`RenameAll::apply`] so the macro reproduces the same
/// output serde produces at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenameAll {
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
pub(crate) enum RenameTarget {
    Field,
    Variant,
}

impl RenameAll {
    /// Apply the rule to an identifier. Mirrors serde's
    /// `apply_to_field` / `apply_to_variant` so the generated names
    /// match what serde will produce on the wire — picking the right
    /// branch from `target`.
    pub(crate) fn apply(self, name: &str, target: RenameTarget) -> String {
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

/// Translate the `rename_all` literal captured by `scan_serde_attrs`
/// into a [`RenameAll`] mode. Returns `RenameAll::None` when no
/// `rename_all` attribute is present, and a compile error when the
/// literal isn't one of the eight recognised modes — the error points
/// at the literal so the highlight lands on what the user typed.
pub(crate) fn rename_all_from_scan(scan: &SerdeScan) -> Result<RenameAll, syn::Error> {
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

/// How a wire name was derived. Carried alongside the wire name so the
/// uniqueness check can report which input produced each conflicting
/// name.
#[derive(Debug, Clone)]
pub(crate) enum WireSource {
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
pub(crate) fn wire_name(
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
pub(crate) fn check_unique_wire_names(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(s: &str) -> Ident {
        syn::Ident::new(s, proc_macro2::Span::call_site())
    }

    // --- wire-name calculation -------------------------------------

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
