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

## Supported field shapes

`#[derive(Schema)]` recognises a fixed scalar set, optionally composed
with `Vec<T>`, `Option<T>`, and the frieze-defined `Maybe<T>` wrapper.
Field types that are themselves `Schema`-deriving structs are emitted as
`$ref` (see [Nested structs and `$ref`](#nested-structs-and-ref)).

### Scalars

| Scalar Rust type | Maps to OAS                                            |
|------------------|--------------------------------------------------------|
| `i32`, `i64`     | `type: integer, format: int32 / int64`                 |
| `u32`, `u64`     | `type: integer, format: int32 / int64, minimum: 0`     |
| `f32`, `f64`     | `type: number, format: float / double`                 |
| `bool`           | `type: boolean`                                        |
| `String`         | `type: string`                                         |

`T` below stands for any of these scalars.

### Composite shapes (presence × nullability)

OpenAPI optionality has two **independent** axes: **presence** controls
whether the field name appears in the schema's `required` array, and
**nullability** controls whether the value may be `null`. The four
combinations map to the following Rust shapes:

| Rust shape                                                            | Presence | Nullability        |
|-----------------------------------------------------------------------|----------|--------------------|
| `T`                                                                   | required | non-nullable       |
| `Option<T>` (serde default)                                           | required | nullable           |
| `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable       |
| `Maybe<T>`                                                            | optional | nullable           |
| `Vec<T>`                                                              | required | array, items as `T` |
| `Vec<Option<T>>`                                                      | required | array, nullable items |
| `Option<Vec<T>>`                                                      | required | nullable array     |
| `Option<Vec<Option<T>>>`                                              | required | nullable array, nullable items |
| `U` (another `Schema`-deriving struct)                                | required | `$ref` to `U`      |
| `Option<U>` (serde default)                                           | required | nullable `$ref`    |
| `Option<U>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable `$ref` |
| `Maybe<U>`                                                            | optional | nullable `$ref`    |
| `Vec<U>`                                                              | required | array of `$ref`    |
| `Vec<Option<U>>`                                                      | required | array of nullable `$ref` |

Notes:

- **`Option<T>` is required-and-nullable by default**, because serde
  emits `None` as `null` and expects the key to be present. This is
  surprising if you read `Option` as "may be omitted" — to get
  **optional + non-nullable**, pair `Option<T>` with the standard
  `#[serde(skip_serializing_if = "Option::is_none")]` attribute. The
  derive inspects that attribute and switches branches accordingly.
- **`Maybe<T>` is the dedicated three-state type** for "missing / null /
  present" — the one combination not expressible by `Option<T>` alone.
  Re-exported as `frieze::Maybe`. Add `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`
  on the field to make missing-key handling work in both directions.
- **Nullability lives on the type tree** (`PropertyType::Nullable`),
  not on the property as a whole. That is how `Vec<Option<T>>` becomes
  an array of nullable items rather than a nullable array.

### Nested structs and `$ref`

A field whose type is another `Schema`-deriving struct is emitted as a
[`PropertyType::Reference(SchemaName)`][reference] in `frieze-model`, and
rendered as `$ref: "#/components/schemas/<Name>"` at the boundary. The
referenced schema must be registered in the same
`SchemasBuilder` — explicit registration is the only mode in Phase 1.
`SchemasBuilder::build()` walks every property's type tree and returns
[`Error::UnresolvedReference`][unresolved] for the first `$ref` whose
target schema isn't registered.

[reference]: ./crates/domain/frieze-model/src/property_type.rs
[unresolved]: ./crates/domain/frieze-model/src/error.rs

Nullable references cannot use a sibling `nullable: true` on a `$ref`
schema (OAS 3.0 ignores it; OAS 3.1 disallows it), so the renderer
wraps them in the version-appropriate composition:

| OAS version | Shape for `Option<U>` (serde default) / `Maybe<U>` |
|-------------|-----------------------------------------------------|
| 3.0         | `allOf: [{$ref}], nullable: true`                  |
| 3.1         | `oneOf: [{$ref}, {type: "null"}]`                  |

Restrictions on user-written types in field positions, enforced as
compile errors:

- **Qualified paths** (`mymod::User`) — bring the type into scope with a
  `use` statement first.
- **Generic type parameters** (`Foo<u32>`) — concrete user types only
  in Phase 1; generics over user schemas are deferred to Phase 1 #11.

### Unsupported shapes (compile error)

The macro rejects ambiguous or unsupported compositions before they
reach the schema-building code:

| Shape                | Reason                                                  |
|----------------------|---------------------------------------------------------|
| `Option<Option<T>>`  | serde flattens nested options.                          |
| `Vec<Vec<T>>`        | nested arrays are deferred to a future PR.              |
| `Vec<Maybe<T>>`      | array elements are always present on the wire; use `Vec<Option<T>>` for nullable items. |
| `Option<Maybe<T>>`   | presence is doubly defined.                             |
| `Maybe<Option<T>>`   | nullability is doubly defined.                          |
| `Maybe<Maybe<T>>`    | nested `Maybe` is not supported.                        |

## Output ordering

`frieze` guarantees specific output ordering even where the OAS treats maps as unordered:

| Output                       | Order                                |
|------------------------------|--------------------------------------|
| `Schema.properties` keys     | Struct field declaration order       |
| `Schema.required` array      | Same order as `properties`           |
| `Schema.enum` array          | Variant declaration order            |
| `#/components/schemas` keys  | Alphabetical by schema name          |

`IndexMap` is used internally where insertion order matters; `BTreeMap` where alphabetical order is desired.

Within a single schema object, keys are emitted in canonical OAS reading order: `$ref`, `type`, `items`, `format`, `minimum`, `allOf`, `oneOf`, `nullable` (3.0 only), `properties`, `required`. A schema object set to a `$ref` is emitted on its own — sibling keys are dropped, matching the OAS rule that `$ref` schemas are treated as leaves.

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
