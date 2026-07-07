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
    frieze-cli           # bin `cargo-frieze`: the `cargo frieze generate` subcommand
  gateway/
    frieze-fs            # Filesystem gateway: package metadata, partials, outputs
    frieze-cargo         # Cargo gateway: schema collection via a scratch crate
  domain/
    frieze-model         # Domain types whose invariants are enforced by the type system
    frieze-usecase       # Boundary conversion, document composition, gateway traits, GenerateOas interactor
  libs/
    frieze-openapi       # Plain representation of the OpenAPI Specification (+ to_yaml)
    frieze-macros        # proc-macro crate
    frieze-wire          # Composition root: injects the concrete gateways into the interactors
    frieze               # User-facing API: Schema / Register traits + SchemasBuilder registry
```

## Dependency direction and invariants

```
frieze-cli     -> frieze-wire, frieze-usecase, frieze-model
frieze-wire    -> frieze-fs, frieze-cargo, frieze-usecase
frieze-fs      -> frieze-usecase, frieze-model, frieze-openapi
frieze-cargo   -> frieze-usecase, frieze-model, frieze-openapi
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
6. Gateway crates (`frieze-fs`, `frieze-cargo`) implement the gateway traits defined in `frieze-usecase`; they do not know about each other.
7. `frieze-usecase` does not depend on any gateway crate — it holds only the trait definitions and the interactors written against them.
8. Concrete gateway types are known only to `frieze-wire` and to the gateway crates themselves; `frieze-cli` obtains the assembled interactor through `frieze-wire` and never names a gateway type.

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

### End-to-end tests

`cargo test --workspace` includes the end-to-end tests in
`crates/apps/frieze-cli/tests/generate.rs`. They run the real
`cargo-frieze` binary against the fixture packages and workspaces
under `crates/apps/frieze-cli/tests/fixtures/` and therefore invoke
real nested cargo builds. Fixture packages linked as path
dev-dependencies (for the byte-equivalence assertions) are pulled into
this repository's workspace by cargo's path-dependency auto-inclusion
(which overrides `exclude`); the standalone error fixtures carry their
own `[workspace]` table instead — see the note in the root
`Cargo.toml`. Further notes:

- The tests are serialized through a lock; each fixture builds into
  its own persistent `target/e2e/<fixture>/` directory, so the first
  run is cold (tens of seconds) and reruns hit the incremental cache.
- The subprocess environment sets `FRIEZE_LOCAL_CRATES_DIR` to the
  checkout root so the generated scratch crate resolves the
  unpublished `frieze` / `frieze-usecase` crates by path. Production
  scratch crates pin the crates.io releases instead.
- Run them alone with `cargo test -p frieze-cli --test generate`.

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
