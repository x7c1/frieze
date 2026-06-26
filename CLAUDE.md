# frieze

`frieze` generates OpenAPI Schema Objects from Rust types via `proc-macros`.

Published as `x7c1/frieze` on GitHub. License: GPL-3.0-or-later. Future crates.io publication is planned.

## Repository layout

This is a Cargo workspace.

```
crates/
  apps/
    frieze-cli           # bin: compose / validate (future)
  domain/
    frieze-model         # Domain types whose invariants are enforced by the type system
    frieze-usecase       # Schema trait, Schemas builder, boundary conversion, use cases
  libs/
    frieze-openapi       # Plain representation of the OpenAPI Specification
    frieze-macros        # proc-macro crate
    frieze               # Facade crate for end users
```

### Dependency direction (must hold)

```
frieze-cli       → frieze-usecase
frieze-usecase   → frieze-model, frieze-openapi
frieze-macros    → frieze-usecase
frieze (facade)  → all of the above
```

### Invariants

1. `frieze-model` depends on nothing else within frieze (and minimally on external crates).
2. `frieze-openapi` does not know about `frieze-model` or `frieze-usecase`.
3. Only `frieze-usecase` performs the boundary conversion between `frieze-openapi` and `frieze-model`.
4. `frieze-model` types use private fields + constructor functions; they cannot be built via struct literals.
5. `frieze-macros` only touches the `Schema` trait defined in `frieze-usecase`; it never constructs `frieze-openapi` or `frieze-model` types directly.

## Terminology

The term **"DTO"** (Data Transfer Object) is **not** used in this repository. Each crate hosts types with distinct responsibilities:

- `frieze-openapi` types are a plain representation of the OAS specification.
- `frieze-model` types are validated domain types that uphold internal invariants.

Lumping them as "DTOs" hides the responsibility difference that the architecture is built upon. Refer to them by their crate-specific roles instead.

## Development workflow

- **Test-first.** For each new feature, start by writing a failing test, then implement the minimum to pass it.
- **1 PR = 1 feature addition = 1 test addition** is the rough granularity. Start from the smallest case and expand incrementally.
- **Unsupported types and structures must produce a compile error.** Better to draw a hard line than to behave partially.

## Output ordering

`frieze` guarantees specific output ordering even where the OAS treats maps as unordered:

| Output                       | Order                                |
|------------------------------|--------------------------------------|
| `Schema.properties` keys     | Struct field declaration order       |
| `Schema.required` array      | Same order as `properties`           |
| `Schema.enum` array          | Variant declaration order            |
| `#/components/schemas` keys  | Alphabetical by schema name          |

`IndexMap` is used internally where insertion order matters; `BTreeMap` where alphabetical order is desired.

## OAS version feature flags

frieze targets exactly ONE OpenAPI Specification version per build. The version is selected by a Cargo feature on the `frieze` facade:

| Feature   | OAS version | Default | Nullability encoding         |
|-----------|-------------|---------|------------------------------|
| `oas-3-0` | 3.0.x       | yes     | `nullable: true`             |
| `oas-3-1` | 3.1.x       | no      | `type: [<base>, "null"]`     |

The two features are mutually exclusive and enforced via `compile_error!` in both `frieze-openapi` and `frieze-usecase`. Build / test with one of:

```
cargo build --workspace --no-default-features --features oas-3-0
cargo test  --workspace --no-default-features --features oas-3-0
cargo build --workspace --no-default-features --features oas-3-1
cargo test  --workspace --no-default-features --features oas-3-1
```

`--all-features` and `--no-default-features` (without picking one) both fail at compile time on purpose.

## Build / Test

```
cargo build --workspace --no-default-features --features oas-3-0
cargo test  --workspace --no-default-features --features oas-3-0
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --no-default-features --features oas-3-0 -- -D warnings
```

Substitute `oas-3-1` for `oas-3-0` to run the same checks against the 3.1 emission path.

## Branch and PR conventions

- `main` is protected: PR required, force-push and deletion forbidden, admin enforcement enabled.
- Squash merge only; branches are deleted on merge.
- Direct commits to `main` are not allowed (admin enforcement is on).
- PR titles follow Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `build:`, `perf:`, `revert:`).
- Snapshot tests use `insta`. Update snapshots intentionally via `cargo insta review`; never blindly accept.
