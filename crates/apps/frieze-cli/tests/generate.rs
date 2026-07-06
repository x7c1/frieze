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
//! - **Byte-equivalence with the library path**: the written YAML is
//!   compared against what `compose` + `to_yaml` produce in-process
//!   for the same partial and the same types.
//!
//! The tests invoke real cargo builds, so they are serialized through
//! a lock and each fixture gets its own build directory
//! (`target/e2e/<fixture>/`) to keep runs independent; the directory
//! persists between runs so local reruns hit the incremental cache.

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
    Command::new(env!("CARGO_BIN_EXE_cargo-frieze"))
        .arg("generate")
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
/// partial, compose the schemas registered by `register`, and render
/// to YAML.
fn library_path_yaml(
    partial: &Path,
    register: impl FnOnce(frieze::SchemasBuilder) -> frieze::SchemasBuilder,
) -> String {
    let partial: frieze_openapi::Document =
        serde_yaml::from_str(&read(partial)).expect("the fixture partial must parse");
    let schemas = register(frieze::SchemasBuilder::new())
        .build()
        .expect("the fixture schemas must build");
    let document =
        frieze_usecase::compose(partial, schemas).expect("the fixture inputs must compose");
    frieze_openapi::to_yaml(&document)
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
