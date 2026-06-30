//! OAS schema-name composition from `module_path!()` + namespace
//! declarations.
//!
//! `#[derive(Schema)]` calls [`compose_schema_name`] from its emitted
//! `Schema::name()` body, passing the derive-site `module_path!()` and
//! the base name computed by existing rules (literal type ident for
//! non-generic, `<Arg>_<Base>` suffix composition for generic). This
//! helper consults the `Namespace` side channel populated by
//! `#[frieze(namespace)]` (under the `inventory` feature) and prefixes
//! the base name with the namespaces whose full paths are prefixes of
//! `module_path`.
//!
//! When the `inventory` feature is off, no namespace declarations can
//! be observed and the helper short-circuits to the base name — every
//! `#[derive(Schema)]` therefore keeps the same OAS key it had before
//! PR 1.5.

/// Compose the OAS schema name for a type derived at `module_path`.
///
/// Walks `module_path` segment-by-segment, looking up each prefix
/// `crate::a::b::c` in the set of namespace declarations collected from
/// `inventory`. Segments whose full prefix matches a declared namespace
/// are kept (in order); segments without a match are dropped. The
/// retained segments are joined by `.` and `.` -joined to `base_name`
/// to produce the OAS key.
///
/// Examples (with `pub mod v1` declared as a namespace in `my_crate`):
///
/// - `compose_schema_name("my_crate::v1", "User") == "v1.User"`
/// - `compose_schema_name("my_crate::v1::detail", "User") == "v1.User"`
///   (`detail` is not declared, dropped)
/// - `compose_schema_name("my_crate::other", "User") == "User"`
///   (no segments retained)
///
/// With the `inventory` feature disabled the namespace set is always
/// empty and this function returns `base_name` unchanged.
pub fn compose_schema_name(module_path: &str, base_name: &str) -> String {
    #[cfg(not(feature = "inventory"))]
    {
        let _ = module_path;
        base_name.to_string()
    }

    #[cfg(feature = "inventory")]
    {
        let namespaces = collected_namespaces();
        let segments: Vec<&str> = module_path.split("::").collect();
        let mut kept: Vec<&str> = Vec::new();
        for i in 1..=segments.len() {
            let prefix = segments[..i].join("::");
            if namespaces.contains(&prefix) {
                kept.push(segments[i - 1]);
            }
        }
        if kept.is_empty() {
            base_name.to_string()
        } else {
            format!("{}.{}", kept.join("."), base_name)
        }
    }
}

/// Lazily build the set of namespace full paths
/// (`format!("{}::{}", parent_path, local_name)` for every
/// `inventory`-submitted `Namespace`) and cache it for the rest of the
/// process.
///
/// `inventory` aggregation is fixed at link time, so the set is stable
/// once observed and no invalidation hook is needed.
#[cfg(feature = "inventory")]
fn collected_namespaces() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        ::inventory::iter::<crate::inventory::Namespace>()
            .map(|ns| format!("{}::{}", ns.parent_path, ns.local_name))
            .collect()
    })
}
