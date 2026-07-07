//! End-to-end tests for `cargo frieze generate`.
//!
//! Each test runs the real `cargo-frieze` binary against a fixture
//! package under `tests/fixtures/`, driving the full pipeline: read
//! `[package.metadata.frieze]`, generate and run the scratch crate
//! through real cargo (which links the fixture and dumps its
//! registered schemas), compose each partial, and write the outputs.
//!
//! Two properties are pinned:
//!
//! - **The linker path works**: the schemas arrive via the inventory
//!   registrations of the *fixture* crate linked into the scratch
//!   binary — nothing in the CLI process itself knows the fixture's
//!   types.
//! - **Byte-equivalence with the library path**: the written YAML or
//!   JSON is compared against what `compose` plus the same
//!   serialization produce in-process for the same partial and the
//!   same types.
//!
//! Configuration mistakes (an unknown key, an unsupported output
//! extension, an `oas-version` mismatch) are covered by dedicated
//! fixtures whose runs fail before any build starts — those tests are
//! cheap and skip the build lock.
//!
//! The tests invoke real cargo builds, so they are serialized through
//! a lock and each fixture gets its own build directory
//! (`target/e2e/<fixture>/`) to keep runs independent; the directory
//! persists between runs so local reruns hit the incremental cache.
//! Tests that share a fixture (the multi-output pair) also share its
//! cache, so only the first of them pays for a cold build.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Serializes the e2e tests: nested cargo builds running in parallel
/// would compete for CPU and rustup/registry locks for no benefit.
static E2E_LOCK: Mutex<()> = Mutex::new(());

/// The frieze repository root (this manifest lives at
/// `crates/apps/frieze-cli/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the repository root must resolve")
}

fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Runs `cargo-frieze generate` inside the fixture directory, with the
/// scratch crate's frieze dependencies redirected to this checkout and
/// a per-fixture build directory.
fn run_generate(fixture: &str) -> std::process::Output {
    run_generate_args(fixture, &[])
}

/// Like [`run_generate`], with extra arguments after `generate`.
fn run_generate_args(fixture: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-frieze"))
        .arg("generate")
        .args(args)
        .current_dir(fixture_dir(fixture))
        .env("FRIEZE_LOCAL_CRATES_DIR", repo_root())
        .env(
            "CARGO_TARGET_DIR",
            repo_root().join("target/e2e").join(fixture),
        )
        .output()
        .expect("cargo-frieze must spawn")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The library-path rendering of the same inputs: parse the fixture's
/// partial and compose the schemas registered by `register`.
fn library_path_document(
    partial: &Path,
    register: impl FnOnce(frieze::SchemasBuilder) -> frieze::SchemasBuilder,
) -> frieze_openapi::Document {
    let partial: frieze_openapi::Document =
        serde_yaml::from_str(&read(partial)).expect("the fixture partial must parse");
    let schemas = register(frieze::SchemasBuilder::new())
        .build()
        .expect("the fixture schemas must build");
    frieze_usecase::compose(partial, schemas).expect("the fixture inputs must compose")
}

/// [`library_path_document`], rendered to YAML.
fn library_path_yaml(
    partial: &Path,
    register: impl FnOnce(frieze::SchemasBuilder) -> frieze::SchemasBuilder,
) -> String {
    frieze_openapi::to_yaml(&library_path_document(partial, register))
}

/// [`library_path_document`], rendered to pretty-printed JSON with a
/// trailing newline — the same serialization the JSON output sink
/// uses.
fn library_path_json(
    partial: &Path,
    register: impl FnOnce(frieze::SchemasBuilder) -> frieze::SchemasBuilder,
) -> String {
    let document = library_path_document(partial, register);
    let mut json =
        serde_json::to_string_pretty(&document).expect("the fixture document must serialize");
    json.push('\n');
    json
}

#[test]
fn single_output_generates_the_declared_document() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = fixture_dir("single-output");
    let output_file = fixture.join("generated/openapi.yaml");
    let _ = std::fs::remove_file(&output_file);

    let output = run_generate("single-output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("generated → ") && stdout.contains("openapi.yaml"),
        "stdout should announce the written path, got:\n{stdout}"
    );

    // Byte-equivalence: the CLI route (scratch crate → inventory dump
    // → compose → sink) and the in-process library route must render
    // identical bytes. `User` pulls `Profile` in transitively, exactly
    // like the inventory walk does.
    let expected = library_path_yaml(&fixture.join("openapi/partial.yaml"), |builder| {
        builder.add::<single_output::User>()
    });
    assert_eq!(read(&output_file), expected);

    // A second run is an idempotent overwrite.
    let rerun = run_generate("single-output");
    assert!(rerun.status.success());
    assert_eq!(read(&output_file), expected);
}

#[test]
fn multi_output_shares_one_collection_across_two_documents() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = fixture_dir("multi-output");
    let public_file = fixture.join("generated/public.yaml");
    let internal_file = fixture.join("generated/internal.yaml");
    let _ = std::fs::remove_file(&public_file);
    let _ = std::fs::remove_file(&internal_file);

    let output = run_generate("multi-output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    // Both outputs are announced, in declaration order.
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected two generated lines:\n{stdout}");
    assert!(lines[0].contains("public.yaml"), "got: {}", lines[0]);
    assert!(lines[1].contains("internal.yaml"), "got: {}", lines[1]);

    // Each document is its own partial composed with the same
    // collected schema set (`Pet` reaches `Owner` transitively).
    for (file, partial) in [
        (&public_file, "openapi/public-partial.yaml"),
        (&internal_file, "openapi/internal-partial.yaml"),
    ] {
        let expected = library_path_yaml(&fixture.join(partial), |builder| {
            builder.add::<multi_output::Pet>()
        });
        assert_eq!(read(file), expected, "for {}", file.display());
    }
    // The two documents differ (different partials), proving the
    // per-output compose actually ran per output.
    assert_ne!(read(&public_file), read(&internal_file));
}

#[test]
fn mixed_formats_covers_json_output_and_oas_3_1_dispatch() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = fixture_dir("mixed-formats");
    let json_file = fixture.join("generated/openapi.json");
    let yaml_file = fixture.join("generated/v31.yaml");
    let _ = std::fs::remove_file(&json_file);
    let _ = std::fs::remove_file(&yaml_file);

    let output = run_generate("mixed-formats");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // The `.json` output is byte-for-byte the library path's
    // pretty-JSON rendering of the same OAS 3.0 partial and types.
    let expected_json = library_path_json(&fixture.join("openapi/v30-partial.yaml"), |builder| {
        builder.add::<mixed_formats::Order>()
    });
    assert_eq!(read(&json_file), expected_json);

    // The second output's partial declares OAS 3.1: the version is
    // lifted from the partial at runtime and the serialization
    // dispatches on it — same collection, different shape.
    let expected_yaml = library_path_yaml(&fixture.join("openapi/v31-partial.yaml"), |builder| {
        builder.add::<mixed_formats::Order>()
    });
    assert_eq!(read(&yaml_file), expected_yaml);
    assert!(
        expected_yaml.starts_with("openapi: 3.1.0"),
        "the YAML output must carry the partial's 3.1 version, got:\n{expected_yaml}"
    );
    // The nullable field proves the 3.1 shape was actually emitted
    // (3.0 would say `nullable: true`).
    assert!(
        expected_yaml.contains("'null'") && !expected_yaml.contains("nullable"),
        "expected the OAS 3.1 null-type encoding, got:\n{expected_yaml}"
    );
}

#[test]
fn metadata_features_gate_what_the_collection_sees() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = fixture_dir("feature-gated");
    let output_file = fixture.join("generated/openapi.yaml");
    let _ = std::fs::remove_file(&output_file);

    let output = run_generate("feature-gated");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // `Gated` sits behind the fixture's non-default `extra` feature;
    // it can only appear because `[package.metadata.frieze] features`
    // was transcribed onto the scratch crate's dependency.
    let written = read(&output_file);
    assert!(
        written.contains("Gated:"),
        "the feature-gated schema must be collected, got:\n{written}"
    );
    let expected = library_path_yaml(&fixture.join("openapi/partial.yaml"), |builder| {
        builder
            .add::<feature_gated::Base>()
            .add::<feature_gated::Gated>()
    });
    assert_eq!(written, expected);
}

#[test]
fn output_flag_generates_only_the_named_output() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Reuses the multi-output fixture (and its build cache).
    let fixture = fixture_dir("multi-output");
    let public_file = fixture.join("generated/public.yaml");
    let internal_file = fixture.join("generated/internal.yaml");
    let _ = std::fs::remove_file(&public_file);
    let _ = std::fs::remove_file(&internal_file);

    let output = run_generate_args("multi-output", &["--output", "public"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Exactly one output is announced and written.
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "expected one generated line:\n{stdout}");
    assert!(lines[0].contains("public.yaml"), "got: {}", lines[0]);
    assert!(public_file.is_file());
    assert!(
        !internal_file.exists(),
        "the filtered-out output must not be written"
    );
}

#[test]
fn an_oas_version_mismatch_is_a_curated_error_without_a_build() {
    // No lock: the run fails at the consistency check, before any
    // cargo invocation.
    let fixture = fixture_dir("version-mismatch");
    let output_file = fixture.join("generated/openapi.yaml");
    let _ = std::fs::remove_file(&output_file);

    let output = run_generate("version-mismatch");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "generate must fail on the version mismatch"
    );
    // The message names the output, the partial's version, and the
    // pinned value.
    for needle in ["`default`", "3.0.3", "oas-version = \"3.1\""] {
        assert!(
            stderr.contains(needle),
            "stderr should contain {needle:?}, got:\n{stderr}"
        );
    }
    assert!(!output_file.exists());
}

#[test]
fn an_unknown_metadata_key_suggests_the_intended_one() {
    let output = run_generate("unknown-key");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "generate must fail on the unknown key"
    );
    assert!(
        stderr.contains("unknown key `parital`") && stderr.contains("did you mean `partial`?"),
        "stderr should reject the typo with a suggestion, got:\n{stderr}"
    );
}

#[test]
fn an_unsupported_output_extension_lists_the_allowed_ones() {
    let output = run_generate("bad-extension");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "generate must fail on the unsupported extension"
    );
    assert!(
        stderr.contains("openapi.txt") && stderr.contains("yaml, yml, json"),
        "stderr should name the path and the allowed extensions, got:\n{stderr}"
    );
}

#[test]
fn an_inventory_disabled_target_fails_with_a_curated_error() {
    let _guard = E2E_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture = fixture_dir("inventory-off");
    let output_file = fixture.join("generated/openapi.yaml");
    let _ = std::fs::remove_file(&output_file);

    let output = run_generate("inventory-off");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "generate must fail for an inventory-disabled target"
    );
    assert!(
        stderr.contains("inventory"),
        "stderr should explain the disabled feature, got:\n{stderr}"
    );
    assert!(
        !output_file.exists(),
        "no output may be written when collection is rejected"
    );
}
