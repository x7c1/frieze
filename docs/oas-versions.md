# OAS versions

`frieze` targets exactly **one** OpenAPI Specification version per
build. The version is selected by a Cargo feature; the `frieze` crate
forwards `oas-3-0` / `oas-3-1` to the crates that encode the version
(`frieze-openapi`, and `frieze-usecase` on top of it).

## Feature flags

| Feature   | OAS version | Default | Nullability encoding         |
|-----------|-------------|---------|------------------------------|
| `oas-3-0` | 3.0.x       | yes     | `nullable: true`             |
| `oas-3-1` | 3.1.x       | no      | `type: [<base>, "null"]`     |

The two features are **mutually exclusive** and enforced via
`compile_error!` in both `frieze-openapi` and `frieze-usecase`.
`--all-features` and `--no-default-features` (without picking one
explicitly) both fail at compile time on purpose — there is no
"version-agnostic" mode.

## Per-version encoding differences

The differences between OAS 3.0 and OAS 3.1 that affect the emitted
schema:

### Nullable scalars and arrays

| Rust shape          | OAS 3.0                                                | OAS 3.1                                           |
|---------------------|--------------------------------------------------------|---------------------------------------------------|
| `Option<T>`         | `{type: <base>, nullable: true}`                       | `{type: [<base>, "null"]}`                        |
| `Option<Vec<T>>`    | `{type: array, items: ..., nullable: true}`            | `{type: [array, "null"], items: ...}`             |
| `Vec<Option<T>>`    | items carry `nullable: true`                           | items carry `type: [<base>, "null"]`              |

### Nullable nested references

| Rust shape                      | OAS 3.0                                       | OAS 3.1                                            |
|---------------------------------|-----------------------------------------------|----------------------------------------------------|
| `Option<U>` (serde default)     | `{allOf: [{$ref: ...}], nullable: true}`      | `{oneOf: [{$ref: ...}, {type: "null"}]}`           |
| `Maybe<U>`                      | `{allOf: [{$ref: ...}], nullable: true}`      | `{oneOf: [{$ref: ...}, {type: "null"}]}`           |

### `description` on a `$ref` property

A `///` doc comment on a field whose type is another `Schema`-deriving
struct or enum needs a per-version encoding because `$ref` schemas
cannot freely carry sibling keys on OAS 3.0:

| Rust shape                  | OAS 3.0                                                          | OAS 3.1                                              |
|-----------------------------|------------------------------------------------------------------|------------------------------------------------------|
| `U` (with `///`)            | `{description, allOf: [{$ref: ...}]}`                            | `{$ref: ..., description}`                           |
| `Option<U>` (with `///`)    | `{description, allOf: [{$ref: ...}], nullable: true}`            | `{description, oneOf: [{$ref: ...}, {type: "null"}]}`|

OAS 3.0 wraps the reference in `allOf` so the description sits on the
outer schema. OAS 3.1 places the description either next to the `$ref`
(plain reference) or on the existing `oneOf` wrap (nullable
reference). Either way, `description` rides on the outermost schema,
never inside the `allOf` / `oneOf` array.

### String enums are version-agnostic

A unit-variant enum derives `type: string, enum: [...]`. The shape
is identical under both `oas-3-0` and `oas-3-1` — neither involves
nullability nor `$ref` siblings, so no per-version wrap is needed.
A nullable enum reference (`Option<EnumType>` field) reuses the same
nullable-reference wrap as a nullable nested-struct reference; see
the table above.

### Internally-tagged enums are version-agnostic

An internally-tagged enum (`#[serde(tag = "...")]` with every variant
a newtype of a `Schema`-implementing struct) derives a `oneOf` schema
with a top-level `discriminator: {propertyName: <tag>}` block. The
shape is identical under both `oas-3-0` and `oas-3-1`. A nullable
`oneOf` reference (`Option<EnumType>` field) reuses the same
nullable-reference wrap as a nullable nested-struct reference; see
the table above.

The `discriminator.mapping` block is deliberately omitted (see
[Internally-tagged enums](field-shapes.md#internally-tagged-enums) in
`field-shapes.md` for the rationale).

### Why the difference

- OAS 3.0 uses a dedicated `nullable: true` keyword. Combined with the
  `$ref`-sibling-ignore rule (sibling keys next to `$ref` are
  silently ignored), nullable references must be wrapped in `allOf`
  to escape that sibling rule.
- OAS 3.1 removes `nullable` entirely in favour of `type` arrays that
  may include `"null"`, and relaxes the `$ref`-sibling rule. Nullable
  references are expressed with `oneOf` against `{type: "null"}`.

## Build / Test

The standard matrix runs the same command set against each version:

```
cargo build  --workspace --no-default-features --features oas-3-0
cargo test   --workspace --no-default-features --features oas-3-0
cargo build  --workspace --no-default-features --features oas-3-1
cargo test   --workspace --no-default-features --features oas-3-1
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --no-default-features --features oas-3-0 -- -D warnings
cargo clippy --workspace --all-targets --no-default-features --features oas-3-1 -- -D warnings
```

Both feature gates must remain green; CI runs the matrix on every PR.
