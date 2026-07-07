//! Where the scratch crate's `frieze` / `frieze-usecase` dependencies
//! come from.
//!
//! The scratch binary and the target crate must resolve the **same**
//! `frieze` package instance: if cargo resolved two, the registrations
//! the target's derive output submits would land in a registry the
//! scratch binary never iterates, and the collection would silently
//! yield zero schemas. The resolution rule closes that hazard up
//! front, before anything is built:
//!
//! - **Path dependency**: the target develops against a local frieze
//!   checkout, so the scratch crate mirrors the same path (and takes
//!   `frieze-usecase` from the same checkout). One path, one instance.
//! - **Registry dependency**: the scratch crate pins the crates.io
//!   release matching this gateway's own version (`=X.Y.Z`; `frieze`
//!   and the CLI ship in lockstep). The target's declared requirement
//!   must therefore be able to match that exact version — a
//!   requirement that cannot is rejected with an error naming both
//!   sides.
//! - **Git dependency**: rejected — a registry pin could never unify
//!   with a git source, and mirroring arbitrary git specs is not
//!   supported.

use std::path::{Path, PathBuf};

use crate::inspect::FriezeDependency;
use crate::Error;

/// The frieze version this gateway was built from. The workspace
/// shares one version across every crate, so this is also the version
/// of the `cargo-frieze` binary and of the `frieze` release it pins.
pub(crate) const CLI_FRIEZE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the scratch crate takes `frieze` and `frieze-usecase` from.
#[derive(Debug, PartialEq)]
pub(crate) enum FriezeCrateSource {
    /// The crates.io releases pinned to `version` exactly.
    PinnedRelease { version: &'static str },
    /// A local frieze checkout: `frieze` mirrored from the target's
    /// path dependency, `frieze-usecase` from the same checkout.
    LocalCheckout {
        frieze_dir: PathBuf,
        usecase_dir: PathBuf,
    },
}

/// Decides the source from the target's declared `frieze` dependency,
/// enforcing the single-instance rule described in the module docs.
pub(crate) fn resolve_frieze_source(
    dependency: &FriezeDependency,
    cli_version: &'static str,
) -> Result<FriezeCrateSource, Error> {
    if let Some(frieze_dir) = &dependency.path {
        return checkout_source(frieze_dir);
    }
    if let Some(source) = &dependency.source {
        if source.starts_with("git+") {
            return Err(Error::PackageInspect {
                message: "the target crate declares its `frieze` dependency from a git \
                          source, which the schema collection cannot mirror: declare a \
                          version requirement matching the installed cargo-frieze, or a \
                          path to a local frieze checkout"
                    .to_string(),
            });
        }
    }
    if !requirement_matches(&dependency.requirement, cli_version)? {
        return Err(Error::FriezeVersionMismatch {
            requirement: dependency.requirement.clone(),
            cli_version: cli_version.to_string(),
        });
    }
    Ok(FriezeCrateSource::PinnedRelease {
        version: cli_version,
    })
}

/// Whether the declared semver requirement can match `version` — the
/// exact version the scratch crate would pin. Matching follows
/// cargo's own semantics via the `semver` crate: `^0.1` accepts
/// `0.1.3`, `^0.1.4` does not accept `0.1.3`, `=0.1.0` accepts only
/// `0.1.0`.
fn requirement_matches(requirement: &str, version: &str) -> Result<bool, Error> {
    let requirement = semver::VersionReq::parse(requirement).map_err(|cause| {
        Error::PackageInspect {
            message: format!(
                "cannot parse the declared frieze version requirement `{requirement}`: {cause}"
            ),
        }
    })?;
    let version = semver::Version::parse(version).map_err(|cause| Error::PackageInspect {
        message: format!("cannot parse the frieze version `{version}`: {cause}"),
    })?;
    Ok(requirement.matches(&version))
}

/// Locates `frieze-usecase` next to a path-declared `frieze`: the path
/// must point at `crates/libs/frieze` inside a checkout of the frieze
/// repository, whose layout fixes `frieze-usecase` at
/// `crates/domain/frieze-usecase`.
fn checkout_source(frieze_dir: &Path) -> Result<FriezeCrateSource, Error> {
    let usecase_dir = frieze_dir
        .ancestors()
        .nth(3)
        .map(|checkout_root| checkout_root.join("crates/domain/frieze-usecase"));
    match usecase_dir {
        Some(usecase_dir) if usecase_dir.join("Cargo.toml").is_file() => {
            Ok(FriezeCrateSource::LocalCheckout {
                frieze_dir: frieze_dir.to_path_buf(),
                usecase_dir,
            })
        }
        _ => Err(Error::PackageInspect {
            message: format!(
                "the target crate's `frieze` path dependency (`{}`) does not sit \
                 inside a checkout of the frieze repository, so the companion \
                 `frieze-usecase` crate the schema collection needs cannot be \
                 located: point the path at `crates/libs/frieze` of a full \
                 checkout, or depend on the crates.io release",
                frieze_dir.display()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependency(
        requirement: &str,
        path: Option<PathBuf>,
        source: Option<&str>,
    ) -> FriezeDependency {
        FriezeDependency {
            uses_default_features: true,
            features: Vec::new(),
            requirement: requirement.to_string(),
            path,
            source: source.map(str::to_owned),
        }
    }

    fn registry_dependency(requirement: &str) -> FriezeDependency {
        dependency(
            requirement,
            None,
            Some("registry+https://github.com/rust-lang/crates.io-index"),
        )
    }

    #[test]
    fn requirements_that_can_match_the_cli_version_are_accepted() {
        for (requirement, version) in [
            ("=0.1.0", "0.1.0"),
            ("^0.1", "0.1.3"),
            ("^0.1.2", "0.1.3"),
            ("*", "0.1.0"),
            (">=0.1, <0.3", "0.2.4"),
        ] {
            assert!(
                requirement_matches(requirement, version).unwrap(),
                "`{requirement}` should accept {version}"
            );
        }
    }

    #[test]
    fn requirements_that_cannot_match_the_cli_version_are_rejected() {
        for (requirement, version) in [
            ("^0.2", "0.1.0"),
            ("=0.1.0", "0.1.1"),
            ("^0.1.4", "0.1.3"),
            ("^0.1", "0.2.0"),
            ("^1", "2.0.0"),
        ] {
            assert!(
                !requirement_matches(requirement, version).unwrap(),
                "`{requirement}` should reject {version}"
            );
        }
    }

    #[test]
    fn a_matching_registry_requirement_pins_the_release() {
        let source = resolve_frieze_source(&registry_dependency("^0.1"), "0.1.3").unwrap();
        assert_eq!(source, FriezeCrateSource::PinnedRelease { version: "0.1.3" });
    }

    #[test]
    fn a_mismatched_registry_requirement_is_rejected() {
        let result = resolve_frieze_source(&registry_dependency("^0.2"), "0.1.3");
        assert!(
            matches!(
                &result,
                Err(Error::FriezeVersionMismatch { requirement, cli_version })
                    if requirement == "^0.2" && cli_version == "0.1.3"
            ),
            "expected the version mismatch to be rejected, got {result:?}"
        );
    }

    #[test]
    fn a_git_dependency_is_rejected() {
        let dependency = dependency(
            "*",
            None,
            Some("git+https://github.com/x7c1/frieze#abcdef"),
        );
        let result = resolve_frieze_source(&dependency, "0.1.0");
        assert!(
            matches!(
                &result,
                Err(Error::PackageInspect { message }) if message.contains("git source")
            ),
            "expected the git dependency to be rejected, got {result:?}"
        );
    }

    #[test]
    fn a_path_dependency_inside_a_checkout_is_mirrored() {
        let checkout = tempfile::tempdir().unwrap();
        let frieze_dir = checkout.path().join("crates/libs/frieze");
        let usecase_dir = checkout.path().join("crates/domain/frieze-usecase");
        std::fs::create_dir_all(&frieze_dir).unwrap();
        std::fs::create_dir_all(&usecase_dir).unwrap();
        std::fs::write(usecase_dir.join("Cargo.toml"), "[package]\n").unwrap();

        let dependency = dependency("*", Some(frieze_dir.clone()), None);
        // The declared requirement is not checked for a path
        // dependency: the mirrored path guarantees a single instance.
        let source = resolve_frieze_source(&dependency, "0.1.0").unwrap();
        assert_eq!(
            source,
            FriezeCrateSource::LocalCheckout {
                frieze_dir,
                usecase_dir,
            }
        );
    }

    #[test]
    fn a_path_dependency_outside_a_checkout_is_rejected() {
        let vendor = tempfile::tempdir().unwrap();
        let frieze_dir = vendor.path().join("vendor/frieze");
        std::fs::create_dir_all(&frieze_dir).unwrap();

        let dependency = dependency("*", Some(frieze_dir), None);
        let result = resolve_frieze_source(&dependency, "0.1.0");
        assert!(
            matches!(
                &result,
                Err(Error::PackageInspect { message })
                    if message.contains("frieze-usecase") && message.contains("vendor/frieze")
            ),
            "expected the non-checkout path to be rejected, got {result:?}"
        );
    }
}
