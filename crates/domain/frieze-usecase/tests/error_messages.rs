//! Pins the exact user-facing wording of the use-case [`Error`]
//! variants and their cause sub-enums.
//!
//! The CLI renders these messages verbatim (prefixed with `error: `),
//! so their wording is part of the user interface: any rewording must
//! be deliberate and show up in a diff of this file. Dynamic path
//! values are interpolated through the same `Display` the message
//! uses, and dynamic sub-errors (`io::Error`, serde errors) are
//! rendered into the expectation, so what each test pins is the
//! curated framing.

use std::io;

use frieze_model::{
    OasVersionCheck, OutputFilePath, OutputName, PackageName, PackageRoot, PartialFilePath,
};
use frieze_usecase::{
    Error, MetadataReadCause, OutputCheckCause, OutputWriteCause, PackageResolveCause,
    PartialReadCause, SchemasCollectCause,
};

fn output_name(name: &str) -> OutputName {
    OutputName::new(name).unwrap()
}

fn package_name(name: &str) -> PackageName {
    PackageName::new(name).unwrap()
}

/// A real package root, partial path, and output path: the parsed
/// path types validate on construction, so the fixtures must exist.
struct Paths {
    _dir: tempfile::TempDir,
    root: PackageRoot,
    partial: PartialFilePath,
    output: OutputFilePath,
}

fn paths() -> Paths {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
    let partial_path = dir.path().join("partial.yaml");
    std::fs::write(&partial_path, "openapi: 3.0.3\n").unwrap();
    Paths {
        root: PackageRoot::try_from_path(dir.path()).unwrap(),
        partial: PartialFilePath::try_from_path(&partial_path).unwrap(),
        output: OutputFilePath::try_from_path(dir.path().join("openapi.yaml")).unwrap(),
        _dir: dir,
    }
}

#[test]
fn unknown_output_name() {
    let error = Error::UnknownOutputName {
        requested: output_name("absent"),
        available: vec![output_name("public"), output_name("internal")],
    };
    assert_eq!(
        error.to_string(),
        "output `absent` is not defined (available: public, internal)"
    );
}

#[test]
fn oas_version_mismatch() {
    let paths = paths();
    let error = Error::OasVersionMismatch {
        output: output_name("public"),
        partial: paths.partial.clone(),
        partial_version: "3.0.3".to_string(),
        expected: OasVersionCheck::V3_1,
    };
    assert_eq!(
        error.to_string(),
        format!(
            "output `public`: the partial document `{}` declares \
             `openapi: 3.0.3`, but the package metadata pins \
             `oas-version = \"3.1\"`: update the partial's `openapi:` \
             field or the pinned `oas-version` so they agree",
            paths.partial
        )
    );
}

#[test]
fn missing_frieze_section() {
    let paths = paths();
    let error = Error::MissingFriezeSection {
        root: paths.root.clone(),
    };
    assert_eq!(
        error.to_string(),
        format!(
            "no `[package.metadata.frieze]` section in the Cargo.toml of \
             `{}`: declare at least one \
             `[[package.metadata.frieze.outputs]]` entry",
            paths.root
        )
    );
}

#[test]
fn no_outputs_defined() {
    let paths = paths();
    let error = Error::NoOutputsDefined {
        root: paths.root.clone(),
    };
    assert_eq!(
        error.to_string(),
        format!(
            "no outputs defined in the Cargo.toml of `{}`: declare at \
             least one `[[package.metadata.frieze.outputs]]` entry",
            paths.root
        )
    );
}

fn resolve(cause: PackageResolveCause) -> String {
    Error::PackageResolve { cause }.to_string()
}

#[test]
fn package_resolve_current_dir() {
    assert_eq!(
        resolve(PackageResolveCause::CurrentDir(io::Error::other(
            "no current directory"
        ))),
        "cannot resolve the target package: cannot determine the current \
         directory: no current directory"
    );
}

#[test]
fn package_resolve_cargo_metadata() {
    // A nonzero exit with captured stderr relays both.
    assert_eq!(
        resolve(PackageResolveCause::CargoMetadata {
            exit_code: Some(101),
            stderr: "error: could not find `Cargo.toml`\n".to_string(),
        }),
        "cannot resolve the target package: cargo metadata failed \
         (exit code 101): error: could not find `Cargo.toml`"
    );
    // A spawn failure has neither an exit code nor captured stderr.
    assert_eq!(
        resolve(PackageResolveCause::CargoMetadata {
            exit_code: None,
            stderr: String::new(),
        }),
        "cannot resolve the target package: cargo metadata failed"
    );
}

#[test]
fn package_resolve_metadata_parse() {
    assert_eq!(
        resolve(PackageResolveCause::MetadataParse {
            message: "missing field `packages`".to_string(),
        }),
        "cannot resolve the target package: cannot interpret the cargo \
         metadata output: missing field `packages`"
    );
}

#[test]
fn package_resolve_requested_package_not_found() {
    assert_eq!(
        resolve(PackageResolveCause::RequestedPackageNotFound {
            requested: package_name("nope"),
            available: vec![package_name("api-v1"), package_name("api-v2")],
        }),
        "cannot resolve the target package: package `nope` is not a \
         member of this workspace (members: api-v1, api-v2)"
    );
}

#[test]
fn package_resolve_default_package_not_found() {
    assert_eq!(
        resolve(PackageResolveCause::DefaultPackageNotFound {
            requested: package_name("nope"),
            available: vec![package_name("api-v1"), package_name("api-v2")],
        }),
        "cannot resolve the target package: `[workspace.metadata.frieze]` \
         declares `package = \"nope\"`, which is not a member of this \
         workspace (members: api-v1, api-v2)"
    );
}

#[test]
fn package_resolve_no_target_package() {
    assert_eq!(
        resolve(PackageResolveCause::NoTargetPackage {
            available: vec![package_name("api-v1"), package_name("api-v2")],
        }),
        "cannot resolve the target package: no target package selected: \
         pass `-p <name>` or declare `package = \"...\"` under \
         `[workspace.metadata.frieze]` in the workspace root Cargo.toml \
         (members: api-v1, api-v2)"
    );
}

#[test]
fn package_resolve_unknown_key() {
    assert_eq!(
        resolve(PackageResolveCause::UnknownKey {
            key: "pacakge".to_string(),
            suggestion: Some("package".to_string()),
        }),
        "cannot resolve the target package: unknown key `pacakge` in \
         `[workspace.metadata.frieze]` (did you mean `package`?)"
    );
    assert_eq!(
        resolve(PackageResolveCause::UnknownKey {
            key: "unrelated".to_string(),
            suggestion: None,
        }),
        "cannot resolve the target package: unknown key `unrelated` in \
         `[workspace.metadata.frieze]`"
    );
}

#[test]
fn package_resolve_unexpected_type() {
    assert_eq!(
        resolve(PackageResolveCause::UnexpectedType {
            key: "package".to_string(),
            expected: "a string",
        }),
        "cannot resolve the target package: key `package` in \
         `[workspace.metadata.frieze]` must be a string"
    );
}

fn metadata_read(cause: MetadataReadCause) -> String {
    let paths = paths();
    let error = Error::MetadataRead {
        root: paths.root.clone(),
        cause,
    };
    let prefix = format!("failed to read the frieze metadata of `{}`: ", paths.root);
    error
        .to_string()
        .strip_prefix(&prefix)
        .expect("the category framing must lead the message")
        .to_string()
}

#[test]
fn metadata_read_causes() {
    assert_eq!(
        metadata_read(MetadataReadCause::CargoManifestRead(io::Error::other(
            "is a directory"
        ))),
        "cannot read Cargo.toml: is a directory"
    );
    assert_eq!(
        metadata_read(MetadataReadCause::CargoManifestParse {
            message: "expected `=` after key".to_string(),
        }),
        "cannot parse Cargo.toml: expected `=` after key"
    );
    assert_eq!(
        metadata_read(MetadataReadCause::MissingPackageTable),
        "Cargo.toml has no `[package]` table"
    );
    assert_eq!(
        metadata_read(MetadataReadCause::UnknownKey {
            key: "parital".to_string(),
            table: "[[package.metadata.frieze.outputs]]".to_string(),
            suggestion: Some("partial".to_string()),
        }),
        "unknown key `parital` in `[[package.metadata.frieze.outputs]]` \
         (did you mean `partial`?)"
    );
    assert_eq!(
        metadata_read(MetadataReadCause::MissingKey {
            key: "output".to_string(),
            table: "[[package.metadata.frieze.outputs]]".to_string(),
        }),
        "missing required key `output` in `[[package.metadata.frieze.outputs]]`"
    );
    assert_eq!(
        metadata_read(MetadataReadCause::UnexpectedType {
            key: "features".to_string(),
            table: "[package.metadata.frieze]".to_string(),
            expected: "an array of strings",
        }),
        "key `features` in `[package.metadata.frieze]` must be an array of strings"
    );
}

#[test]
fn partial_read_causes() {
    let paths = paths();
    let render = |cause: PartialReadCause| {
        Error::PartialRead {
            path: paths.partial.clone(),
            cause,
        }
        .to_string()
    };
    let prefix = format!(
        "failed to read the partial OAS document `{}`: ",
        paths.partial
    );
    assert_eq!(
        render(PartialReadCause::NotFound),
        format!("{prefix}file not found")
    );
    assert_eq!(
        render(PartialReadCause::PermissionDenied),
        format!("{prefix}permission denied")
    );
    assert_eq!(
        render(PartialReadCause::Io(io::Error::other("interrupted"))),
        format!("{prefix}interrupted")
    );
    // The serde parse variants relay the parser's own rendering after
    // a named framing; the parser text itself is not ours to pin.
    let yaml_error = serde_yaml::from_str::<serde_yaml::Value>("{").unwrap_err();
    let expected = format!("{prefix}YAML parse error: {yaml_error}");
    assert_eq!(render(PartialReadCause::YamlParse(yaml_error)), expected);
    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let expected = format!("{prefix}JSON parse error: {json_error}");
    assert_eq!(render(PartialReadCause::JsonParse(json_error)), expected);
}

fn schemas_collect(cause: SchemasCollectCause) -> String {
    Error::SchemasCollect { cause }.to_string()
}

#[test]
fn schemas_collect_causes() {
    assert_eq!(
        schemas_collect(SchemasCollectCause::ScratchGenerate(io::Error::other(
            "disk full"
        ))),
        "failed to collect schemas from the target crate: cannot generate \
         the scratch crate: disk full"
    );
    assert_eq!(
        schemas_collect(SchemasCollectCause::PackageInspect {
            message: "package `my-api` has no lib target".to_string(),
        }),
        "failed to collect schemas from the target crate: cannot \
         interpret the package layout: package `my-api` has no lib target"
    );
    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let expected = format!(
        "failed to collect schemas from the target crate: cannot parse \
         the collected schemas: {json_error}"
    );
    assert_eq!(
        schemas_collect(SchemasCollectCause::ScratchStdoutParse(json_error)),
        expected
    );
}

#[test]
fn schemas_collect_cargo_invocation() {
    // The build log already streamed to the terminal: nothing is
    // repeated, only the exit code is reported.
    assert_eq!(
        schemas_collect(SchemasCollectCause::CargoInvocation {
            exit_code: Some(101),
            stderr: String::new(),
        }),
        "failed to collect schemas from the target crate: cargo \
         invocation failed (exit code 101)"
    );
    // A spawn failure carries whatever the OS reported.
    assert_eq!(
        schemas_collect(SchemasCollectCause::CargoInvocation {
            exit_code: None,
            stderr: "No such file or directory".to_string(),
        }),
        "failed to collect schemas from the target crate: cargo \
         invocation failed: No such file or directory"
    );
}

#[test]
fn schemas_collect_inventory_disabled() {
    assert_eq!(
        schemas_collect(SchemasCollectCause::InventoryDisabled),
        "failed to collect schemas from the target crate: the target \
         crate disables the frieze `inventory` feature, so its schemas \
         cannot be collected: re-enable it by removing \
         `default-features = false` from the frieze dependency, or by \
         adding \"inventory\" to its `features` list"
    );
}

#[test]
fn output_write_causes() {
    let paths = paths();
    let render = |cause: OutputWriteCause| {
        Error::OutputWrite {
            path: paths.output.clone(),
            cause,
        }
        .to_string()
    };
    let prefix = format!("failed to write the output `{}`: ", paths.output);
    assert_eq!(
        render(OutputWriteCause::ParentDirCreate(io::Error::other(
            "read-only filesystem"
        ))),
        format!("{prefix}cannot create the parent directory: read-only filesystem")
    );
    assert_eq!(
        render(OutputWriteCause::Write(io::Error::other("disk full"))),
        format!("{prefix}disk full")
    );
    assert_eq!(
        render(OutputWriteCause::PermissionDenied),
        format!("{prefix}permission denied")
    );
    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let expected = format!("{prefix}JSON serialize error: {json_error}");
    assert_eq!(
        render(OutputWriteCause::SerializeJson(json_error)),
        expected
    );
}

#[test]
fn output_check_causes() {
    let paths = paths();
    let render = |cause: OutputCheckCause| {
        Error::OutputCheck {
            path: paths.output.clone(),
            cause,
        }
        .to_string()
    };
    let prefix = format!("failed to check the output `{}`: ", paths.output);
    assert_eq!(
        render(OutputCheckCause::PermissionDenied),
        format!("{prefix}permission denied")
    );
    assert_eq!(
        render(OutputCheckCause::Read(io::Error::other("interrupted"))),
        format!("{prefix}interrupted")
    );
    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let expected = format!("{prefix}JSON serialize error: {json_error}");
    assert_eq!(
        render(OutputCheckCause::SerializeJson(json_error)),
        expected
    );
}
