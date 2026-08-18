# frieze — agent operating rules

`frieze` generates OpenAPI Schema Objects from Rust types via `proc-macros`.
Published as `x7c1/frieze` on GitHub. License: GPL-3.0-or-later.

This file is for AI agents working in this repository. End-user-facing
specification lives in [`docs/`](docs/) and [`README.md`](README.md).

## Architecture

The workspace layout, dependency direction, crate-boundary invariants,
and terminology rules are specified in
[`docs/architecture/README.md`](docs/architecture/README.md), expanded
here:

@docs/architecture/README.md

## Development workflow

- **Test-first.** For each new feature, start by writing a failing test, then implement the minimum to pass it.
- **1 PR = 1 feature addition = 1 test addition** is the rough granularity. Start from the smallest case and expand incrementally.
- **Unsupported types and structures must produce a compile error.** Better to draw a hard line than to behave partially.

## Build / Test matrix

The OAS version (3.0 / 3.1) is per-document runtime data, so one test
run covers both output shapes. Three feature axes remain (`inventory`,
`uuid1`, `chrono04`), giving a 13-command fmt / build / clippy / test
matrix. The command list and its rationale live in
[`docs/oas-versions/README.md` § Build / Test](docs/oas-versions/README.md#build--test);
CI (`.github/workflows/ci.yml`) runs the same steps on every PR. Run
the full matrix locally before opening a PR.

### End-to-end tests

`cargo test --workspace` includes the end-to-end tests in
`crates/apps/frieze-cli/tests/generate.rs`, which run the real
`cargo-frieze` binary against the fixtures under
`crates/apps/frieze-cli/tests/fixtures/` and therefore invoke real
nested cargo builds (the first run is cold, tens of seconds; reruns
hit the incremental cache). The details — what the fixtures pin, the
build lock, per-fixture build directories, and how fixture packages
join this workspace — live in that file's module doc comment and the
workspace note in the root `Cargo.toml`. Run them alone with
`cargo test -p frieze-cli --test generate`.

## Branch and PR conventions

- `main` is protected: PR required, force-push and deletion forbidden, admin enforcement enabled.
- Squash merge only; branches are deleted on merge.
- Direct commits to `main` are not allowed (admin enforcement is on).
- PR titles follow Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `build:`, `perf:`, `revert:`).
- Snapshot tests use `insta`. Update snapshots intentionally via `cargo insta review`; never blindly accept.

## OSS hygiene (no upstream-private references)

This repository is published as OSS. Comments, docstrings, commit messages, PR
titles, PR bodies, and documentation files MUST be self-contained against the
contents of this repository — they MUST NOT depend on, link to, or quote
terminology that lives only in any other (upstream / private / planning)
repository.

In particular, do NOT introduce:

- Numbered design labels that are only defined in an external plan document
  (e.g. `branch ①`, `branch ④`, `N1`–`N4`, `case ②`).
- Cross-repository URLs or paths that point outside this repository.
- Quoted decisions or rationales that the reader cannot resolve from files
  inside this repository.

When a concept is referenced inside this repository, prefer the canonical
wording defined in [`docs/field-shapes/README.md`](docs/field-shapes/README.md),
[`docs/output-ordering/README.md`](docs/output-ordering/README.md),
[`docs/oas-versions/README.md`](docs/oas-versions/README.md),
[`docs/architecture/README.md`](docs/architecture/README.md), or this
`CLAUDE.md` itself — not an abbreviation that only makes sense in an
upstream tracker.

All artifacts pushed to this repository (code, comments, commit messages, PR
descriptions, documentation) are written in English.

## Documentation pointers

When you change behaviour, also update the matching specification file:

- Workspace layout, dependency direction, crate-boundary invariants, terminology → [`docs/architecture/README.md`](docs/architecture/README.md)
- Supported field shapes, compile-error categories, `Maybe<T>` handling, nested-struct (`$ref`) behaviour → [`docs/field-shapes/README.md`](docs/field-shapes/README.md)
- Output ordering, canonical key order, the empty-container omission rule → [`docs/output-ordering/README.md`](docs/output-ordering/README.md)
- OAS feature flags, per-version encoding differences, the build/test matrix → [`docs/oas-versions/README.md`](docs/oas-versions/README.md)
- Library usage surface (crate roles, `compose`, `inventory` collection) → [`docs/library/README.md`](docs/library/README.md)
- CLI behaviour (`cargo frieze generate` configuration, workspace resolution, `--check`) → [`docs/cli/README.md`](docs/cli/README.md)
- End-user-visible behaviour or quick-start surface → also check [`README.md`](README.md)
