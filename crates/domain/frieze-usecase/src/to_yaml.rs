//! Renders a [`frieze_model::Schemas`] collection as a YAML string.

use frieze_model::Schemas;

use crate::to_value::to_value;

/// Renders the schemas as a YAML string in the canonical key order.
///
/// Equivalent to `serde_yaml::to_string(&to_value(schemas)).unwrap()`. The
/// unwrap is safe because the `serde_yaml::Value` returned by
/// [`to_value`] is always a finite mapping of strings and sequences.
///
/// # Scalar style for multi-line strings
///
/// Any string value that contains an embedded `\n` is emitted as a YAML
/// literal block scalar with the strip chomping indicator (`|-`) rather
/// than as a double-quoted scalar with `\n` escapes. The two forms are
/// wire-equivalent (the parsed string is identical), but the block
/// scalar form preserves the line structure of `description` payloads —
/// CommonMark paragraphs, the bullet lists frieze synthesises for
/// `enum` variants, multi-line examples — verbatim on the page, which
/// matters because OpenAPI specs are read by humans far more often than
/// they are parsed by tools.
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
pub fn to_yaml(schemas: &Schemas) -> String {
    serde_yaml::to_string(&to_value(schemas))
        .expect("frieze: serializing a finite Value mapping to YAML cannot fail")
}
