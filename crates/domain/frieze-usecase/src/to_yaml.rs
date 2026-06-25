//! Renders a [`frieze_model::Schemas`] collection as a YAML string.

use frieze_model::Schemas;

use crate::to_value::to_value;

/// Renders the schemas as a YAML string in the canonical key order.
///
/// Equivalent to `serde_yaml::to_string(&to_value(schemas)).unwrap()`. The
/// unwrap is safe because the `serde_yaml::Value` returned by
/// [`to_value`] is always a finite mapping of strings and sequences.
pub fn to_yaml(schemas: &Schemas) -> String {
    serde_yaml::to_string(&to_value(schemas))
        .expect("frieze: serializing a finite Value mapping to YAML cannot fail")
}
