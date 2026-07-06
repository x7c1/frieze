# frieze — agent operating rules

`frieze` generates OpenAPI Schema Objects from Rust types via `proc-macros`.
Published as `x7c1/frieze` on GitHub. License: GPL-3.0-or-later.

This file is for AI agents working in this repository. End-user-facing
specification lives in [`docs/`](docs/) and [`README.md`](README.md).

## Repository layout

This is a Cargo workspace.

```
crates/
  apps/
    frieze-cli           # bin: compose / validate (future)
  domain/
    frieze-model         # Domain types whose invariants are enforced by the type system
    frieze-usecase       # Boundary conversion and document composition (compose / from_schemas)
  libs/
    frieze-openapi       # Plain representation of the OpenAPI Specification (+ to_yaml)
    frieze-macros        # proc-macro crate
    frieze               # User-facing API: Schema / Register traits + SchemasBuilder registry
```

## Dependency direction and invariants

```
frieze-cli     -> frieze-usecase
frieze-usecase -> frieze-model, frieze-openapi
frieze         -> frieze-model, frieze-macros
```

(`frieze-macros` has no runtime dependency on the other crates: the
tokens it emits resolve through `::frieze::__private`. `frieze` also
dev-depends on `frieze-openapi` / `frieze-usecase` for its integration
tests.)

1. `frieze-model` depends on nothing else within frieze (and minimally on external crates).
2. `frieze-openapi` does not know about `frieze-model` or `frieze-usecase`.
3. Only `frieze-usecase` performs the boundary conversion between `frieze-openapi` and `frieze-model`.
4. `frieze-model` types use private fields + constructor functions; they cannot be built via struct literals.
5. `frieze-macros` only touches the `Schema` / `Register` traits and the `__private` helpers defined in `frieze`; it never constructs `frieze-openapi` types, and reaches `frieze-model` constructors only through `::frieze::__private`.

## Terminology

The term **"DTO"** is **not** used here. `frieze-openapi` types are a
plain representation of the OAS specification; `frieze-model` types are
validated domain types that uphold internal invariants. Lumping them as
"DTOs" hides the responsibility difference the architecture is built
upon — refer to them by their crate-specific roles instead.

## Development workflow

- **Test-first.** For each new feature, start by writing a failing test, then implement the minimum to pass it.
- **1 PR = 1 feature addition = 1 test addition** is the rough granularity. Start from the smallest case and expand incrementally.
- **Unsupported types and structures must produce a compile error.** Better to draw a hard line than to behave partially.

## Build / Test matrix

The OAS version (3.0 / 3.1) is per-document runtime data, so one test
run covers both output shapes. The only feature axis is `inventory`
(on by default; the `--no-default-features` runs keep the opt-out
path for no_std / WASM-leaning consumers green):

```
cargo fmt --all -- --check
cargo build  --workspace
cargo build  --workspace --no-default-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo test   --workspace
cargo test   --workspace --no-default-features
```

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
wording defined in [`docs/field-shapes.md`](docs/field-shapes.md),
[`docs/output-ordering.md`](docs/output-ordering.md),
[`docs/oas-versions.md`](docs/oas-versions.md), or this `CLAUDE.md` itself —
not an abbreviation that only makes sense in an upstream tracker.

All artifacts pushed to this repository (code, comments, commit messages, PR
descriptions, documentation) are written in English.

## Documentation pointers

When you change behaviour, also update the matching specification file:

- Supported field shapes, compile-error categories, `Maybe<T>` handling, nested-struct (`$ref`) behaviour → [`docs/field-shapes.md`](docs/field-shapes.md)
- Output ordering, canonical key order, the empty-container omission rule → [`docs/output-ordering.md`](docs/output-ordering.md)
- OAS feature flags, per-version encoding differences, the build/test matrix → [`docs/oas-versions.md`](docs/oas-versions.md)
- End-user-visible behaviour or quick-start surface → also check [`README.md`](README.md)
