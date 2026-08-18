# frieze

Generate OpenAPI documents from Rust types via `#[derive(Schema)]`.

- **Derive-based schemas** — `#[derive(Schema)]` turns plain Rust
  structs and enums into OpenAPI Schema Objects, with unsupported
  shapes rejected at compile time rather than half-supported.
- **Presence and nullability, kept orthogonal** — `Option<T>`,
  `#[serde(skip_serializing_if)]`, and `Maybe<T>` cover all four
  optional × nullable field shapes.
- **serde-faithful output** — `///` doc comments become
  `description`s, `rename` / `rename_all` are honoured, and every
  serde attribute frieze cannot encode into the schema is a compile
  error, so the schema always matches what goes on the wire.
- **Composes with hand-written documents** — generated schemas merge
  into a partial OAS document (`info`, `paths`, vendor extensions)
  without disturbing the rest.
- **A cargo subcommand** — `cargo frieze generate` writes the declared
  documents from `Cargo.toml` metadata alone, with a `--check` mode
  that keeps the committed documents fresh in CI.
- **OAS 3.0 and 3.1 side by side** — the version is per-document
  runtime data, not a Cargo feature; one program can emit both.
- **No schema list to maintain** — every `#[derive(Schema)]` type is
  collected automatically; deriving a type is registering it (with
  explicit registration and a no_std / WASM opt-out when you need
  them).
- **`uuid` / `chrono` support** — opt-in features map `Uuid`,
  `DateTime<Tz>`, and `NaiveDate` to `format: uuid` / `date-time` /
  `date`.

## Status

frieze is in early development: the public API may still shift, and
the crates are not yet published on crates.io. Until then, depend on
`frieze` via a path dependency into a checkout of this repository, and
install the CLI with `cargo install --path crates/apps/frieze-cli`.

## Getting started

```rust
use frieze::{Schema, SchemasBuilder};
use frieze_openapi::{Info, Version};

/// A registered user of the system.
#[derive(Schema)]
struct User {
    /// The user's id.
    id: i64,
    /// Short profile text — present but nullable, serde's default
    /// reading of `Option<T>`.
    bio: Option<String>,
}

let schemas = SchemasBuilder::new()
    .add::<User>()
    .build()
    .expect("schemas build should succeed");
let document = frieze_usecase::from_schemas(
    Info { title: "My API".into(), version: "1.0.0".into(), ..Default::default() },
    schemas,
    Version::V3_0,
);
println!("{}", frieze_openapi::to_yaml(&document));
```

The same pipeline is available with no generation code at all:
`cargo frieze generate` builds the declared documents straight from
`Cargo.toml` metadata — see [`docs/cli.md`](docs/cli.md).

## Documentation

| File                                                 | Topic                                                              |
|------------------------------------------------------|--------------------------------------------------------------------|
| [`docs/library.md`](docs/library.md)                 | Library walk-through: crate roles, `compose`, schema collection    |
| [`docs/cli.md`](docs/cli.md)                         | The `cargo frieze generate` subcommand, workspaces, CI `--check`   |
| [`docs/field-shapes.md`](docs/field-shapes.md)       | Field types, presence/nullability, `uuid` / `chrono` support       |
| [`docs/oas-versions.md`](docs/oas-versions.md)       | OAS version handling and version differences                       |
| [`docs/output-ordering.md`](docs/output-ordering.md) | Output ordering guarantees                                         |
| [`RELEASING.md`](RELEASING.md)                       | Release procedure and version policy                               |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
