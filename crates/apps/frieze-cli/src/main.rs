//! `frieze-cli` — placeholder entry point.
//!
//! The CLI surface (compose / validate / etc.) is intentionally empty in
//! Phase 1; the binary exists so the workspace structure is in place for
//! later work.

fn main() {
    // Intentionally empty in Phase 1. A `--help` flag and subcommands will be
    // added in a later PR once there is something to expose.
    let _ = frieze_usecase::SchemasBuilder::new();
}
