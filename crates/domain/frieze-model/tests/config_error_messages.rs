//! Pins the exact user-facing wording of every [`ConfigError`]
//! variant.
//!
//! These messages reach the terminal verbatim through the CLI, so
//! their wording is part of the user interface: any rewording must be
//! deliberate and show up in a diff of this file, not slip in as a
//! side effect. Variants that embed a dynamic sub-error (`io::Error`)
//! pin the framing around it by rendering the same sub-error into the
//! expectation.

use std::io;
use std::path::PathBuf;

use frieze_model::{ConfigError, OutputName};

fn output_name(name: &str) -> OutputName {
    OutputName::new(name).unwrap()
}

#[test]
fn package_root_not_directory() {
    let error = ConfigError::PackageRootNotDirectory {
        got: PathBuf::from("/work/api"),
    };
    assert_eq!(
        error.to_string(),
        "package root `/work/api` is not a directory"
    );
}

#[test]
fn missing_cargo_toml() {
    let error = ConfigError::MissingCargoToml {
        root: PathBuf::from("/work/api"),
    };
    assert_eq!(error.to_string(), "no Cargo.toml found in `/work/api`");
}

#[test]
fn package_root_canonicalize() {
    let error = ConfigError::PackageRootCanonicalize {
        got: PathBuf::from("/work/api"),
        cause: io::Error::other("component vanished"),
    };
    assert_eq!(
        error.to_string(),
        "cannot canonicalize package root `/work/api`: component vanished"
    );
}

#[test]
fn partial_file_not_found() {
    let error = ConfigError::PartialFileNotFound {
        got: PathBuf::from("/work/api/openapi/partial.yaml"),
    };
    assert_eq!(
        error.to_string(),
        "partial OAS document `/work/api/openapi/partial.yaml` does not \
         exist: create it, or fix the `partial` path declared in \
         `[[package.metadata.frieze.outputs]]`"
    );
}

#[test]
fn output_parent_unwritable() {
    let error = ConfigError::OutputParentUnwritable {
        got: PathBuf::from("/work/api/generated/openapi.yaml"),
        cause: io::Error::other("read-only filesystem"),
    };
    assert_eq!(
        error.to_string(),
        "cannot create the parent directory of output path \
         `/work/api/generated/openapi.yaml`: read-only filesystem"
    );
}

#[test]
fn unsupported_file_extension() {
    let error = ConfigError::UnsupportedFileExtension {
        got: PathBuf::from("/work/api/openapi.txt"),
        allowed: &["yaml", "yml", "json"],
    };
    assert_eq!(
        error.to_string(),
        "unsupported file extension on `/work/api/openapi.txt` \
         (expected one of: yaml, yml, json)"
    );
}

#[test]
fn unsupported_format_extension() {
    let error = ConfigError::UnsupportedFormatExtension {
        got: "txt".to_string(),
        allowed: &["yaml", "yml", "json"],
    };
    assert_eq!(
        error.to_string(),
        "unsupported format extension `txt` (expected one of: yaml, yml, json)"
    );
}

#[test]
fn output_name_empty() {
    assert_eq!(
        ConfigError::OutputNameEmpty.to_string(),
        "output name must not be empty"
    );
}

#[test]
fn output_name_invalid_char() {
    let error = ConfigError::OutputNameInvalidChar {
        got: "pub/lic".to_string(),
        at: 3,
        ch: '/',
    };
    assert_eq!(
        error.to_string(),
        "output name `pub/lic` contains invalid character `/` at byte 3 \
         (allowed characters: a-z, 0-9, `_`, `-`)"
    );
}

#[test]
fn package_name_invalid() {
    let error = ConfigError::PackageNameInvalid {
        got: "1api".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "`1api` is not a valid cargo package name (non-empty, characters \
         a-z, A-Z, 0-9, `_`, `-`, must not start with a digit)"
    );
}

#[test]
fn cargo_feature_name_invalid() {
    let error = ConfigError::CargoFeatureNameInvalid {
        got: "-extra".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "`-extra` is not a valid cargo feature name (starts with an ASCII \
         alphanumeric or `_`, followed by ASCII alphanumerics, `_`, `-`, \
         `.`, or `+`)"
    );
}

#[test]
fn oas_version_check_invalid() {
    let error = ConfigError::OasVersionCheckInvalid {
        got: "3.2".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "invalid `oas-version` value `3.2` (expected \"3.0\" or \"3.1\")"
    );
}

#[test]
fn output_name_collision() {
    let error = ConfigError::OutputNameCollision {
        name: output_name("public"),
    };
    assert_eq!(
        error.to_string(),
        "output name `public` is declared more than once"
    );
}

#[test]
fn output_path_collision() {
    let error = ConfigError::OutputPathCollision {
        path: PathBuf::from("/work/api/generated/openapi.yaml"),
        used_by: vec![output_name("public"), output_name("internal")],
    };
    assert_eq!(
        error.to_string(),
        "output path `/work/api/generated/openapi.yaml` is used by more \
         than one output: public, internal"
    );
}
