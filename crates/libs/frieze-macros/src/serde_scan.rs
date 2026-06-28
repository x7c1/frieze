//! Single-pass scanner for `#[serde(...)]` attributes.
//!
//! [`scan_serde_attrs`] walks every meta entry exactly once per item and
//! either records a supported value into [`SerdeScan`] or raises a
//! compile error for an unsupported (DEFER) form. Routing every diagnostic
//! through a single entry point ensures that an unsupported attribute on
//! one field cannot slip past one branch and be silently honored by
//! another.

use syn::Attribute;

/// Position context for [`scan_serde_attrs`]. Currently unused for
/// branching — every position rejects the same DEFER attribute set —
/// but kept so future changes can vary the diagnostic per site without
/// reshuffling call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SerdePosition {
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
pub(crate) struct SerdeScan {
    /// `#[serde(rename = "literal")]`, if present. Direction-split forms
    /// `#[serde(rename(serialize = ..., deserialize = ...))]` produce an
    /// error during scanning and therefore never reach this field.
    pub(crate) rename: Option<(String, proc_macro2::Span)>,
    /// `#[serde(rename_all = "literal")]`, if present. Same
    /// direction-split rejection as `rename`.
    pub(crate) rename_all: Option<(String, proc_macro2::Span)>,
    /// `true` when the field carries the bare `#[serde(default)]`.
    /// Custom-default forms like `#[serde(default = "path")]` are
    /// consumed but do **not** set this flag — the `Maybe<T>` gate
    /// requires the bare form.
    pub(crate) default_bare: bool,
    /// The literal string of the `skip_serializing_if = "..."`
    /// attribute, if present. The two values currently recognised by
    /// frieze are `"Option::is_none"` (switches an `Option<T>` field to
    /// optional / non-nullable) and `"Maybe::is_missing"` (part of the
    /// required `Maybe<T>` attribute pair). Other predicates are stored
    /// verbatim but have no effect on the generated schema.
    pub(crate) skip_serializing_if: Option<String>,
}

const DIRECTION_SPLIT_RENAME_MSG: &str = "frieze: `#[serde(rename(serialize = ..., deserialize = ...))]` produces different wire names for serialize and deserialize. A single OAS schema cannot represent both. Use a symmetric `#[serde(rename = \"...\")]` instead, or split the type into request- and response-shaped variants.";

const DIRECTION_SPLIT_RENAME_ALL_MSG: &str = "frieze: `#[serde(rename_all(serialize = ..., deserialize = ...))]` produces different wire names for serialize and deserialize. A single OAS schema cannot represent both. Use a symmetric `#[serde(rename_all = \"...\")]` instead, or split the type into request- and response-shaped variants.";

/// Walk every `#[serde(...)]` attribute and classify each nested meta
/// entry into the [`SerdeScan`] record. DEFER attributes raise a compile
/// error here so a single scan covers both data extraction and the
/// unsupported-attribute check.
///
/// The `position` argument is currently informational — every Rust site
/// rejects the same DEFER set — but kept so future versions can tighten
/// the rules per site without rerouting calls.
pub(crate) fn scan_serde_attrs(
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

/// Returns `true` when the scanned attributes carry
/// `#[serde(skip_serializing_if = "Option::is_none")]`. Replaces the
/// older one-purpose attribute walker; the value comes from the central
/// scan so DEFER attributes can't slip past on the same field.
pub(crate) fn is_option_skip_predicate(scan: &SerdeScan) -> bool {
    scan.skip_serializing_if.as_deref() == Some("Option::is_none")
}

/// Returns `true` when the field carries the `Maybe<T>` attribute pair
/// (`#[serde(default, skip_serializing_if = "Maybe::is_missing")]`).
/// Drives the `Maybe<T>` attribute validator.
pub(crate) fn has_maybe_attribute_pair(scan: &SerdeScan) -> bool {
    scan.default_bare && scan.skip_serializing_if.as_deref() == Some("Maybe::is_missing")
}
