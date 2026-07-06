//! The cargo-backed [`SchemasCollector`] implementation.

use std::path::Path;
use std::process::{Command, Stdio};

use frieze_model::{PackageMetadata, PackageRoot};
use frieze_openapi::Components;
use frieze_usecase::{Error as UsecaseError, Result, SchemasCollectCause, SchemasCollector};

use crate::inspect::{cargo_bin, inspect_package};
use crate::scratch::prepare_scratch;
use crate::Error;

/// Collects the schemas the target crate registers by generating a
/// scratch crate that links it, running the scratch binary via cargo,
/// and parsing the canonical components dump from its stdout.
///
/// The flow per collection:
///
/// 1. `cargo metadata` locates the build directory, the workspace
///    root, and the target's lib crate name, and exposes how the
///    target declares its `frieze` dependency.
/// 2. A target crate that opts out of the `inventory` feature is
///    rejected up front: its derive output submits no registrations,
///    so a scratch build could only ever produce an empty document.
///    The feature is never force-enabled behind the user's back.
/// 3. The scratch crate is (re)generated under
///    `<target_directory>/frieze/<package>/` and its lockfile is
///    seeded from the workspace's.
/// 4. `cargo run` builds and runs the scratch binary with stdout
///    captured (the components dump) and stderr passed straight
///    through to the terminal, so build progress and compile errors
///    reach the user exactly as cargo emits them.
#[derive(Debug, Default)]
pub struct CargoSchemasCollector;

impl CargoSchemasCollector {
    pub fn new() -> Self {
        Self
    }
}

impl SchemasCollector for CargoSchemasCollector {
    fn collect(&self, root: &PackageRoot, metadata: &PackageMetadata) -> Result<Components> {
        collect_components(root, metadata).map_err(|error| UsecaseError::SchemasCollect {
            cause: collect_cause(error),
        })
    }
}

/// The whole collection flow in terms of the gateway's internal error;
/// the trait boundary above maps it into the semantic cause.
fn collect_components(
    root: &PackageRoot,
    metadata: &PackageMetadata,
) -> std::result::Result<Components, Error> {
    let inspection = inspect_package(root, metadata.package_name())?;
    match &inspection.frieze_dependency {
        None => {
            return Err(Error::PackageInspect {
                message: format!(
                    "package `{}` does not depend on `frieze`; \
                     add it to [dependencies] and derive `frieze::Schema` \
                     on the types to expose",
                    metadata.package_name()
                ),
            })
        }
        Some(dependency) if !dependency.inventory_enabled() => {
            return Err(Error::InventoryDisabled);
        }
        Some(_) => {}
    }
    let scratch_dir = prepare_scratch(root, metadata.package_name(), &inspection)?;
    let stdout = run_scratch(&scratch_dir)?;
    serde_json::from_slice(&stdout).map_err(Error::StdoutParse)
}

/// Builds and runs the scratch binary, returning its captured stdout.
///
/// stderr is inherited: cargo's build log — including any compile
/// error in the target crate — streams to the user's terminal
/// unmodified, so the error variant for a failed run carries only the
/// exit code.
fn run_scratch(scratch_dir: &Path) -> std::result::Result<Vec<u8>, Error> {
    let output = Command::new(cargo_bin())
        .arg("run")
        .arg("--manifest-path")
        .arg(scratch_dir.join("Cargo.toml"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|cause| Error::CargoInvocation {
            exit_code: None,
            stderr: cause.to_string(),
        })?;
    if !output.status.success() {
        return Err(Error::CargoInvocation {
            exit_code: output.status.code(),
            stderr: String::new(),
        });
    }
    Ok(output.stdout)
}

/// Maps the gateway's internal failures to the semantic
/// [`SchemasCollectCause`] the use-case layer exposes.
fn collect_cause(error: Error) -> SchemasCollectCause {
    match error {
        Error::ScratchIo(cause) | Error::ScratchTemplateWrite(cause) => {
            SchemasCollectCause::ScratchGenerate(cause)
        }
        Error::PackageInspect { message } => SchemasCollectCause::PackageInspect { message },
        Error::InventoryDisabled => SchemasCollectCause::InventoryDisabled,
        Error::CargoInvocation { exit_code, stderr } => {
            SchemasCollectCause::CargoInvocation { exit_code, stderr }
        }
        Error::StdoutParse(cause) => SchemasCollectCause::ScratchStdoutParse(cause),
    }
}
