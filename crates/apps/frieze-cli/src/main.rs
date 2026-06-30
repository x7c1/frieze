//! `frieze-cli` — placeholder entry point.
//!
//! The CLI surface (compose / validate / etc.) is intentionally empty for
//! now; the binary exists so the workspace structure is in place for
//! later work.

fn main() {
    // Intentionally empty for now. A `--help` flag and subcommands will be
    // added once there is something to expose.
    let _ = frieze_usecase::SchemasBuilder::new();
}
