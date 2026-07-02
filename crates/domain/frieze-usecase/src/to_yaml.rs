//! Renders an [`Document`] as a YAML string.

use frieze_openapi::Document;

/// Renders a complete [`Document`] as a YAML string in the canonical
/// key order.
///
/// Equivalent to `serde_yaml::to_string(document).unwrap()`. The unwrap
/// is safe because every `Serialize` impl in the `Document` tree is
/// total — every reachable type either uses an auto-derive or a
/// handwritten impl whose `serialize_*` calls cannot fail on a
/// well-formed value.
///
/// This is the format-neutral output entry point on the YAML side. For
/// JSON, callers route the same `Document` through `serde_json`
/// directly:
///
/// ```ignore
/// let doc: Document = frieze::compose(partial, schemas)?;
/// let yaml = frieze::to_yaml(&doc);
/// let json = serde_json::to_string_pretty(&doc)?;
/// ```
///
/// # Scalar style for multi-line strings
///
/// Any string value in the document that contains an embedded `\n` is
/// emitted as a YAML literal block scalar with the strip chomping
/// indicator (`|-`) rather than a double-quoted scalar with `\n`
/// escapes. The two forms are wire-equivalent (the parsed string is
/// identical), but the block scalar form preserves the line structure
/// of `description` payloads — CommonMark paragraphs, the bullet lists
/// frieze synthesises for `enum` variants, multi-line examples —
/// verbatim on the page, which matters because OpenAPI specs are read
/// by humans far more often than they are parsed by tools.
///
/// This rule applies to *every* multi-line string anywhere in the
/// emitted YAML (not only `description`): if a field's value contains a
/// newline, it is emitted as a block scalar. Single-line strings retain
/// the default scalar form chosen by the underlying YAML emitter.
///
/// The behaviour is delegated to `serde_yaml`'s serializer, whose
/// `serialize_str` impl selects `ScalarStyle::Literal` whenever the
/// input contains a `\n`. Replacing the YAML backend in the future
/// would need to preserve this rule explicitly.
pub fn to_yaml(document: &Document) -> String {
    serde_yaml::to_string(document)
        .expect("frieze: serializing a finite Document tree to YAML cannot fail")
}
