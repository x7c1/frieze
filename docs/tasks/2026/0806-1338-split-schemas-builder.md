---
status: completed
pipeline_phase: null
plan: null
base_ref: null
perspectives: [rust-module-structure, consistency, minimalism, clarity]
retries_remaining: 1
check_command: "cargo fmt --all -- --check && cargo build --workspace && cargo build --workspace --no-default-features && cargo build -p frieze --features uuid1 && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --no-default-features -- -D warnings && cargo clippy -p frieze --all-targets --features uuid1 -- -D warnings && cargo test --workspace && cargo test --workspace --no-default-features && cargo test -p frieze --features uuid1 && test -d crates/libs/frieze/src/schemas_builder && [ -z \"$(find crates/libs/frieze/src -name '*.rs' -exec awk 'END{if(NR>400)print FILENAME}' {} \\;)\" ]"
assignee: null
branch: task/0806-1338-split-schemas-builder
created_at: 2026-08-06T13:38:52Z
updated_at: 2026-08-06T14:57:00Z
---

# refactor: split schemas_builder.rs into a directory module

## Overview

`crates/libs/frieze/src/schemas_builder.rs` has grown to ~600 lines — more
than three times the next-largest file in the crate — and every recent
feature has appended to it (the reserved-name check, the `Uuid` scalar).
Its contents fall into three clearly separable responsibilities:

- the `SchemasBuilder` type and its `impl` (registration funnel,
  deferred-error recording, `build()`) — lines ~23–183,
- the unresolved-reference walk: three free functions
  (`check_one_of_variants_target_struct_schemas`,
  `first_unresolved_in_schema`, `first_unresolved_reference`) — lines
  ~184–291,
- a ~300-line `#[cfg(test)] mod tests`.

Convert the single file into a **directory module** `schemas_builder/` so
each responsibility lives in its own file and future feature work appends
to a focused file instead of one ever-growing one. Suggested decomposition
(adjust names to what reads naturally, keeping one public type per file):

- the module root holding `SchemasBuilder` (the only `pub` type — its path
  `frieze::SchemasBuilder` and the `mod schemas_builder;` declaration in
  `lib.rs` must keep working unchanged),
- a sibling file for the reference-walk helpers (crate-private),
- tests distributed next to what they test (each file's `#[cfg(test)]`
  block covers its own contents; a test exercising the builder end to end
  belongs with the builder).

This is a **pure reorganization**: no behavior change, no public-API
change, no wording change to error messages or doc comments beyond what
moving code strictly requires (e.g. adjusted `use` paths, module docs for
the new files). Snapshot literals and error-message pin tests must pass
untouched.

## Acceptance criteria

### Automated (pipeline-verified)

- [x] `crates/libs/frieze/src/schemas_builder/` exists as a directory
      module and no `.rs` file under `crates/libs/frieze/src/` exceeds
      400 lines (both enforced by gates appended to `check_command`).
- [x] `frieze::SchemasBuilder` remains importable at the same path — the
      whole existing test suite (unit, integration, snapshot, e2e) passes
      without modifying any test expectation (`cargo test --workspace`,
      `--no-default-features`, and `-p frieze --features uuid1`).
- [x] The reference-walk helpers live in their own file, not in the module
      root, and remain crate-private (no new `pub` items appear in the
      crate; `cargo doc` surface is unchanged — verified by the compile +
      clippy gates and the review phases).

## Out of scope

- Any behavior or API change, however small.
- Restructuring other files (`lib.rs`, `primitive_schema_impls.rs`, …) —
  this task covers only `schemas_builder.rs`.
- Renaming existing types, functions, or error variants.
