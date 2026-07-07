//! The cargo-backed [`PackageResolver`] implementation.
//!
//! `cargo frieze generate` may be invoked from anywhere inside a
//! cargo project — the workspace root, a member's directory, or any
//! subdirectory. One `cargo metadata` call (run from the invocation
//! directory, so cargo's own manifest discovery walks up to the
//! enclosing workspace) yields everything the resolution needs: the
//! workspace root, the member list, and the workspace-level
//! `[workspace.metadata.frieze]` declaration.
//!
//! The target package is then selected with this precedence:
//!
//! 1. the explicitly requested package (`-p <name>`), always;
//! 2. the member whose directory contains the current directory, when
//!    that directory is not the workspace root itself — inside a
//!    member you get that member, exactly like `cargo build`;
//! 3. the package declared under `[workspace.metadata.frieze]`
//!    (`package = "..."` in the workspace root `Cargo.toml`);
//! 4. the root package, when the workspace root is itself a package;
//!    or the sole member of a single-member workspace — this is what
//!    keeps a plain single-package crate working with no
//!    configuration at all;
//! 5. otherwise (a virtual workspace with several members and no
//!    declaration) the resolution fails with an error listing the
//!    members and both selection mechanisms.
//!
//! The `[workspace.metadata.frieze]` table itself is validated on
//! every run — an unknown key or a declaration naming a non-member is
//! rejected even when an explicit `-p` would bypass it, so a broken
//! declaration cannot linger silently.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use frieze_model::{PackageName, PackageRoot};
use frieze_usecase::{
    suggest_key, Error as UsecaseError, PackageResolveCause, PackageResolver, Result,
};
use serde_json::Value;

use crate::inspect::cargo_bin;

/// The keys the `[workspace.metadata.frieze]` table may carry today.
///
/// Anything else is rejected with a curated error instead of being
/// silently ignored — a typo like `pacakge` must not turn into "the
/// wrong package was generated".
const KNOWN_WORKSPACE_KEYS: &[&str] = &["package"];

/// Resolves the target package through `cargo metadata`, starting from
/// the process's current directory.
#[derive(Debug, Default)]
pub struct CargoPackageResolver;

impl CargoPackageResolver {
    pub fn new() -> Self {
        Self
    }
}

impl PackageResolver for CargoPackageResolver {
    fn resolve(&self, package: Option<&PackageName>) -> Result<PackageRoot> {
        resolve_package(package).map_err(|cause| UsecaseError::PackageResolve { cause })
    }
}

/// The whole resolution flow in terms of the semantic cause; the trait
/// boundary above wraps it into the boundary variant. The subprocess
/// call and the pure parsing/selection steps are split below so the
/// latter stay unit-testable; the composed flow is exercised by the
/// end-to-end tests.
fn resolve_package(
    package: Option<&PackageName>,
) -> std::result::Result<PackageRoot, PackageResolveCause> {
    let cwd = std::env::current_dir().map_err(PackageResolveCause::CurrentDir)?;
    let json = workspace_metadata_json(&cwd)?;
    let view = parse_workspace(&json)?;
    let member = select_member(&view, package, &normalized(&cwd))?;
    PackageRoot::try_from_path(&member.root).map_err(|error| {
        // The member root came out of cargo metadata, so a failing
        // construction means the filesystem changed underneath us.
        PackageResolveCause::MetadataParse {
            message: error.to_string(),
        }
    })
}

/// What the resolution needs to know about the enclosing workspace.
///
/// A plain single-package crate is the degenerate case: one member
/// whose root is the workspace root, and no declaration.
#[derive(Debug)]
struct WorkspaceView {
    /// The workspace root directory.
    workspace_root: PathBuf,
    /// Every workspace member (for a single-package crate: just it).
    members: Vec<Member>,
    /// The `package = "..."` declaration of
    /// `[workspace.metadata.frieze]`, when present.
    default_package: Option<PackageName>,
}

/// One workspace member: its package name and the directory holding
/// its `Cargo.toml`.
#[derive(Debug)]
struct Member {
    name: PackageName,
    root: PathBuf,
}

/// Runs `cargo metadata` from `cwd`, letting cargo's manifest
/// discovery find the enclosing workspace. A failed invocation carries
/// cargo's captured stderr verbatim; the resolver does not reformat
/// it.
fn workspace_metadata_json(cwd: &Path) -> std::result::Result<Value, PackageResolveCause> {
    let output = Command::new(cargo_bin())
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|cause| PackageResolveCause::CargoMetadata {
            exit_code: None,
            stderr: cause.to_string(),
        })?;
    if !output.status.success() {
        return Err(PackageResolveCause::CargoMetadata {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|cause| PackageResolveCause::MetadataParse {
        message: format!("cargo metadata did not produce valid JSON: {cause}"),
    })
}

/// Extracts the [`WorkspaceView`] from a parsed `cargo metadata`
/// document (invoked with `--no-deps`, so `packages` lists exactly the
/// workspace members). Split from the subprocess call so the
/// extraction logic is testable against canned documents.
fn parse_workspace(json: &Value) -> std::result::Result<WorkspaceView, PackageResolveCause> {
    let workspace_root = json["workspace_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| parse_error("cargo metadata output has no `workspace_root` field"))?;
    let members = json["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(parse_member)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if members.is_empty() {
        return Err(parse_error(
            "cargo metadata reported a workspace without members",
        ));
    }
    let default_package = parse_workspace_table(json)?;
    Ok(WorkspaceView {
        workspace_root: normalized(&workspace_root),
        members: members
            .into_iter()
            .map(|member| Member {
                name: member.name,
                root: normalized(&member.root),
            })
            .collect(),
        default_package,
    })
}

/// Extracts one member from a `packages` entry.
fn parse_member(package: &Value) -> std::result::Result<Member, PackageResolveCause> {
    let name = package["name"]
        .as_str()
        .ok_or_else(|| parse_error("a package in the cargo metadata output has no `name`"))?;
    let name = PackageName::new(name).map_err(|error| PackageResolveCause::MetadataParse {
        message: error.to_string(),
    })?;
    let root = package["manifest_path"]
        .as_str()
        .and_then(|manifest| Path::new(manifest).parent())
        .ok_or_else(|| {
            parse_error(&format!(
                "package `{name}` has no usable `manifest_path` in the cargo metadata output"
            ))
        })?;
    Ok(Member {
        name,
        root: root.to_path_buf(),
    })
}

/// Parses the `[workspace.metadata.frieze]` table from the top-level
/// `metadata` field of the `cargo metadata` output, enforcing its
/// schema: only the `package` key exists, its value is a string.
fn parse_workspace_table(
    json: &Value,
) -> std::result::Result<Option<PackageName>, PackageResolveCause> {
    let frieze = &json["metadata"]["frieze"];
    if frieze.is_null() {
        return Ok(None);
    }
    let table = frieze.as_object().ok_or_else(|| {
        parse_error(
            "`[workspace.metadata] frieze` in the workspace root Cargo.toml must be a table",
        )
    })?;
    if let Some(key) = table
        .keys()
        .find(|key| !KNOWN_WORKSPACE_KEYS.contains(&key.as_str()))
    {
        return Err(PackageResolveCause::UnknownKey {
            key: key.clone(),
            suggestion: suggest_key(key, KNOWN_WORKSPACE_KEYS).map(str::to_string),
        });
    }
    let Some(value) = table.get("package") else {
        return Ok(None);
    };
    let name = value
        .as_str()
        .ok_or_else(|| PackageResolveCause::UnexpectedType {
            key: "package".to_string(),
            expected: "a string",
        })?;
    let name = PackageName::new(name).map_err(|error| PackageResolveCause::MetadataParse {
        message: error.to_string(),
    })?;
    Ok(Some(name))
}

fn parse_error(message: &str) -> PackageResolveCause {
    PackageResolveCause::MetadataParse {
        message: message.to_string(),
    }
}

/// Applies the selection precedence documented on the module.
///
/// The workspace declaration is validated up front — even a run that
/// selects its target another way rejects a declaration naming a
/// non-member, so the configuration cannot rot unnoticed.
fn select_member<'a>(
    view: &'a WorkspaceView,
    requested: Option<&PackageName>,
    cwd: &Path,
) -> std::result::Result<&'a Member, PackageResolveCause> {
    let default = match &view.default_package {
        Some(name) => Some(find_member(view, name).ok_or_else(|| {
            PackageResolveCause::DefaultPackageNotFound {
                requested: name.clone(),
                available: member_names(view),
            }
        })?),
        None => None,
    };
    if let Some(name) = requested {
        return find_member(view, name).ok_or_else(|| {
            PackageResolveCause::RequestedPackageNotFound {
                requested: name.clone(),
                available: member_names(view),
            }
        });
    }
    if let Some(member) = innermost_member_containing(view, cwd) {
        return Ok(member);
    }
    if let Some(member) = default {
        return Ok(member);
    }
    if let Some(root_package) = view
        .members
        .iter()
        .find(|member| member.root == view.workspace_root)
    {
        return Ok(root_package);
    }
    if let [sole_member] = view.members.as_slice() {
        return Ok(sole_member);
    }
    Err(PackageResolveCause::NoTargetPackage {
        available: member_names(view),
    })
}

fn find_member<'a>(view: &'a WorkspaceView, name: &PackageName) -> Option<&'a Member> {
    view.members.iter().find(|member| member.name == *name)
}

/// The workspace member whose directory contains `cwd`, preferring the
/// innermost match for nested member layouts. The root package is
/// deliberately not matched this way: every path in the workspace is
/// inside its directory, so matching it here would shadow the
/// workspace-level declaration whenever a root package exists. It is
/// picked up by the later fallback instead.
fn innermost_member_containing<'a>(view: &'a WorkspaceView, cwd: &Path) -> Option<&'a Member> {
    view.members
        .iter()
        .filter(|member| member.root != view.workspace_root && cwd.starts_with(&member.root))
        .max_by_key(|member| member.root.components().count())
}

/// The member names for an error listing, in stable (sorted) order.
fn member_names(view: &WorkspaceView) -> Vec<PackageName> {
    let mut names: Vec<PackageName> = view.members.iter().map(|m| m.name.clone()).collect();
    names.sort();
    names
}

/// Canonicalizes `path` when it exists, so containment checks are
/// immune to symlinked spellings (`/tmp` vs `/private/tmp`, symlinked
/// checkouts). A path that does not exist is kept as-is.
fn normalized(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canned two-member workspace with a root package, mirroring
    /// the shape `cargo metadata --no-deps` reports.
    fn canned_workspace(metadata: Value) -> Value {
        serde_json::json!({
            "workspace_root": "/work/ws",
            "packages": [
                {
                    "name": "ws-root",
                    "manifest_path": "/work/ws/Cargo.toml",
                },
                {
                    "name": "api",
                    "manifest_path": "/work/ws/api/Cargo.toml",
                },
                {
                    "name": "shared",
                    "manifest_path": "/work/ws/shared/Cargo.toml",
                },
            ],
            "metadata": metadata,
        })
    }

    fn view_with_default(default: Option<&str>) -> WorkspaceView {
        let metadata = match default {
            Some(name) => serde_json::json!({ "frieze": { "package": name } }),
            None => Value::Null,
        };
        parse_workspace(&canned_workspace(metadata)).unwrap()
    }

    /// Drops the root package, turning the canned workspace into a
    /// virtual one.
    fn virtual_view(mut view: WorkspaceView) -> WorkspaceView {
        view.members
            .retain(|member| member.name.as_str() != "ws-root");
        view
    }

    fn name(value: &str) -> PackageName {
        PackageName::new(value).unwrap()
    }

    #[test]
    fn parses_members_and_the_workspace_declaration() {
        let view = view_with_default(Some("api"));
        assert_eq!(view.workspace_root, PathBuf::from("/work/ws"));
        let names: Vec<&str> = view.members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["ws-root", "api", "shared"]);
        assert_eq!(view.members[1].root, PathBuf::from("/work/ws/api"));
        assert_eq!(view.default_package, Some(name("api")));
    }

    #[test]
    fn a_missing_workspace_table_means_no_default() {
        assert_eq!(view_with_default(None).default_package, None);
        // `[workspace.metadata]` without a `frieze` table, likewise.
        let json = canned_workspace(serde_json::json!({ "other-tool": {} }));
        assert_eq!(parse_workspace(&json).unwrap().default_package, None);
    }

    #[test]
    fn an_unknown_workspace_key_is_rejected_with_a_suggestion() {
        let json = canned_workspace(serde_json::json!({ "frieze": { "pacakge": "api" } }));
        let result = parse_workspace(&json);
        assert!(
            matches!(
                &result,
                Err(PackageResolveCause::UnknownKey { key, suggestion })
                    if key == "pacakge" && suggestion.as_deref() == Some("package")
            ),
            "expected the typo key to be rejected, got {result:?}"
        );
    }

    #[test]
    fn a_non_string_package_declaration_is_rejected() {
        let json = canned_workspace(serde_json::json!({ "frieze": { "package": 1 } }));
        let result = parse_workspace(&json);
        assert!(
            matches!(
                &result,
                Err(PackageResolveCause::UnexpectedType { key, expected })
                    if key == "package" && *expected == "a string"
            ),
            "expected the integer declaration to be rejected, got {result:?}"
        );
    }

    #[test]
    fn an_explicit_request_wins_over_everything() {
        let view = view_with_default(Some("api"));
        // Requested from inside another member's directory: the
        // request still wins.
        let member =
            select_member(&view, Some(&name("shared")), Path::new("/work/ws/api/src")).unwrap();
        assert_eq!(member.name.as_str(), "shared");
    }

    #[test]
    fn an_unknown_request_lists_the_members() {
        let view = view_with_default(None);
        let result = select_member(&view, Some(&name("nope")), Path::new("/work/ws"));
        match result {
            Err(PackageResolveCause::RequestedPackageNotFound {
                requested,
                available,
            }) => {
                assert_eq!(requested.as_str(), "nope");
                let names: Vec<&str> = available.iter().map(PackageName::as_str).collect();
                assert_eq!(names, ["api", "shared", "ws-root"]);
            }
            other => panic!("expected the unknown request to be rejected, got {other:?}"),
        }
    }

    #[test]
    fn inside_a_member_directory_that_member_wins() {
        // Even against a workspace declaration naming another member:
        // location is more specific than the workspace-level default.
        let view = view_with_default(Some("api"));
        let member = select_member(&view, None, Path::new("/work/ws/shared/src/nested")).unwrap();
        assert_eq!(member.name.as_str(), "shared");
        // The member's own root counts as inside it.
        let member = select_member(&view, None, Path::new("/work/ws/shared")).unwrap();
        assert_eq!(member.name.as_str(), "shared");
    }

    #[test]
    fn at_the_workspace_root_the_declaration_wins() {
        let view = view_with_default(Some("api"));
        let member = select_member(&view, None, Path::new("/work/ws")).unwrap();
        assert_eq!(member.name.as_str(), "api");
        // Below the root but outside every member (e.g. a scripts/
        // directory), likewise.
        let member = select_member(&view, None, Path::new("/work/ws/scripts")).unwrap();
        assert_eq!(member.name.as_str(), "api");
    }

    #[test]
    fn a_declaration_naming_a_non_member_is_rejected_even_with_an_explicit_request() {
        let json = canned_workspace(serde_json::json!({ "frieze": { "package": "ghost" } }));
        let view = parse_workspace(&json).unwrap();
        let result = select_member(&view, Some(&name("api")), Path::new("/work/ws"));
        assert!(
            matches!(
                &result,
                Err(PackageResolveCause::DefaultPackageNotFound { requested, .. })
                    if requested.as_str() == "ghost"
            ),
            "expected the dangling declaration to be rejected, got {result:?}"
        );
    }

    #[test]
    fn without_a_declaration_the_root_package_is_the_fallback() {
        let view = view_with_default(None);
        let member = select_member(&view, None, Path::new("/work/ws")).unwrap();
        assert_eq!(member.name.as_str(), "ws-root");
    }

    #[test]
    fn a_single_member_workspace_needs_no_configuration() {
        let mut view = virtual_view(view_with_default(None));
        view.members.retain(|member| member.name.as_str() == "api");
        let member = select_member(&view, None, Path::new("/work/ws")).unwrap();
        assert_eq!(member.name.as_str(), "api");
    }

    #[test]
    fn a_virtual_workspace_with_nothing_declared_is_an_error() {
        let view = virtual_view(view_with_default(None));
        let result = select_member(&view, None, Path::new("/work/ws"));
        match result {
            Err(PackageResolveCause::NoTargetPackage { available }) => {
                let names: Vec<&str> = available.iter().map(PackageName::as_str).collect();
                assert_eq!(names, ["api", "shared"]);
            }
            other => panic!("expected the ambiguous workspace to be rejected, got {other:?}"),
        }
    }
}
