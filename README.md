# frieze

Generate OpenAPI Schema Objects from Rust types via `#[derive(Schema)]`.

> Status: early development. Public API may shift.

## Quick start

`frieze` derives a schema description from a plain Rust struct and
hands you back a complete OpenAPI document you can render to YAML or
JSON.

```rust
use frieze::{Schema, SchemasBuilder};
use frieze_model::Maybe;
use frieze_openapi::Info;
use serde::{Deserialize, Serialize};

/// A registered user of the system.
#[derive(Schema, Serialize, Deserialize)]
struct User {
    /// The user's id. Required + non-nullable.
    id: i64,
    // Required + nullable: the key must appear, but its value may be
    // `null`. This is the serde default for `Option<T>`.
    bio: Option<String>,
    // Optional + non-nullable: the key may be omitted entirely, but if
    // present must hold a value. Triggered by the `skip_serializing_if`
    // attribute below.
    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
    // Optional + nullable: the key may be missing, present-with-null, or
    // present-with-value. Use `Maybe<T>` for this three-state shape.
    #[serde(default, skip_serializing_if = "Maybe::is_missing")]
    avatar_url: Maybe<String>,
    // Required array; per-element nullability is independent.
    tags: Vec<Option<String>>,
}

let schemas = SchemasBuilder::new()
    .add::<User>()
    .build()
    .expect("schemas build should succeed");
let document = frieze_usecase::from_schemas(
    Info { title: "My API".into(), version: "1.0.0".into(), ..Default::default() },
    schemas,
    frieze_openapi::Version::V3_0,
);
println!("{}", frieze_openapi::to_yaml(&document));
```

The `frieze` crate owns the pieces user types interact with — the
`Schema` / `Register` traits, `#[derive(Schema)]`, and the
`SchemasBuilder` registry. Document assembly lives in the companion
crates: `frieze-openapi` holds the OAS wire types (`Document`, `Info`,
`Version`, ...) and `to_yaml`, `frieze-usecase` holds `compose` /
`from_schemas`, and `frieze-model` holds the validated domain types
(`Maybe`, `Schemas`, `Error`, ...). Depend on the ones you use
directly.

The same `Document` value is format-neutral — render it to JSON
through serde directly when needed:

```rust
let json = serde_json::to_string_pretty(&document)?;
```

When the user already has a hand-written OAS document fragment
(`info`, `paths`, `tags`, vendor extensions), `frieze_usecase::compose`
merges schemas into it without disturbing the rest:

```rust
let partial: frieze_openapi::Document = serde_yaml::from_str(&yaml)?;
let document = frieze_usecase::compose(partial, schemas)?;
```

`compose` rejects partials that already carry entries under
`components.schemas`: the Rust types collected by `SchemasBuilder` are
the single source of truth for that slot.

A `///` doc comment on the struct or on any field becomes the OAS
`description` for that schema or property — written once in Rust,
rendered automatically. See
[Doc comments to `description`](docs/field-shapes.md#doc-comments-to-description)
for the full mapping (enum-level and per-variant doc-comments are
composed into the enum schema's `description`).

`#[serde(rename = "literal")]` on a field or variant and
`#[serde(rename_all = "<mode>")]` on a struct or enum container are
honoured so that the schema's `properties` keys, `required` entries,
and `enum` values match the names serde will produce on the wire. The
precedence rule, the uniqueness check, and the list of serde
attributes frieze cannot encode into a single OAS schema (and
therefore rejects at compile time) are documented under
[Wire names](docs/field-shapes.md#wire-names-rename-and-rename_all).

## CLI quick start: `cargo frieze generate`

The `frieze-cli` crate ships the same pipeline as a cargo subcommand,
so a crate can get its complete OAS document without writing any
generation code — no hand-written dump binary, no `build.rs`. From
zero to a generated document:

**1. Install the CLI.** The binary is named `cargo-frieze`, which
makes it available as a `cargo frieze` subcommand:

```console
$ cargo install frieze-cli
```

> Until the crates are published on crates.io, install from a checkout
> instead (`cargo install --path crates/apps/frieze-cli`) and declare
> `frieze` as a path dependency into that same checkout — the
> crates.io route below assumes published releases.

**2. Derive the schemas.** Add `frieze` to `[dependencies]` and put
`#[derive(Schema)]` on the types to expose, exactly as in the
[quick start](#quick-start) above. The installed CLI and the declared
`frieze` dependency must agree on the version — see the version
matching note below.

**3. Write the partial document** — the hand-written half (`info`,
`paths`, tags, vendor extensions) with **no** `components.schemas`
(the Rust types are the single source of truth for that slot):

```yaml
# openapi/partial.yaml
openapi: 3.0.3
info:
  title: My API
  version: 0.1.0
paths:
  /users/{id}:
    get:
      responses:
        "200":
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
```

**4. Declare the outputs** in `Cargo.toml`. Even a single output uses
the array form:

```toml
[[package.metadata.frieze.outputs]]
name    = "default"
partial = "openapi/partial.yaml"
output  = "openapi/openapi.yaml"
```

**5. Generate.** From the package directory (or any directory inside
it — see [Workspaces](#workspaces) for how the target package is
picked):

```console
$ cargo frieze generate
   Compiling my-api v0.1.0 (...)
   Compiling frieze-scratch-my-api v0.0.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s)
generated → openapi/openapi.yaml
```

In CI, run the same command with `--check` to fail the build whenever
a committed document is out of date — see
[the CI section](#ci-verify-the-committed-documents-with---check).

Details worth knowing:

- **Multiple outputs.** Declare several `[[outputs]]` entries (unique
  `name`s, unique `output` paths) to generate e.g. a public and an
  internal document from one crate; the schemas are collected once and
  composed into each partial. `cargo frieze generate --output <name>`
  restricts a run to the one output declared under `<name>`.
- **Paths** in the declaration resolve relative to the package's
  `Cargo.toml`. The output **format** follows the output path's
  extension: `.yaml` / `.yml` for YAML, `.json` for JSON.
- **Cargo features.** The `[package.metadata.frieze]` parent table may
  declare `features = ["..."]` — cargo features to enable on your
  crate while its schemas are collected, shared by every output. Types
  behind `#[cfg(feature = "...")]` only reach the document when the
  feature is listed here (or is on by default).
- **OAS version.** The generated document always follows its partial's
  `openapi:` field — 3.0 and 3.1 partials can live side by side. The
  parent table may additionally pin `oas-version = "3.0"` (or
  `"3.1"`) as a consistency check: a partial outside that major.minor
  line fails the run with a clear error before anything is built or
  written.
- **Unknown keys are errors.** Any key the frieze tables do not define
  is rejected — with a "did you mean ...?" suggestion when it looks
  like a typo — rather than silently ignored.
- **Byte-equivalence.** The CLI applies no transformation of its own:
  the written document is byte-for-byte what the library path
  (`frieze_usecase::compose` + `frieze_openapi::to_yaml`) produces for
  the same partial and types.
- **How it works.** The CLI generates a small *scratch* crate under
  `target/frieze/<package>/` that links your crate, runs it via cargo
  (so incremental builds apply and `cargo clean` removes everything),
  and receives the collected schemas from its stdout. Build output
  streams to your terminal exactly as cargo emits it; generation only
  ever runs when you invoke `cargo frieze generate`.
- **The `inventory` feature is required** on your crate's `frieze`
  dependency (it is on by default). A crate that opts out via
  `default-features = false` gets a clear error — the CLI never
  re-enables the feature behind your back; use the library path
  (`SchemasBuilder::add`) for inventory-less setups.
- **The frieze versions must match.** The schemas are collected with
  the exact frieze version the installed CLI ships with (the scratch
  crate pins `frieze = "=X.Y.Z"`), because two different frieze
  versions in one build would resolve as two separate instances and
  the collection would come back empty. A declared `frieze`
  requirement that cannot match the installed `cargo-frieze`'s
  version therefore fails up front, with an error naming both
  versions — after upgrading one side, upgrade the other to match. A
  crate that declares `frieze` as a **path dependency** into a
  checkout of this repository is exempt: the scratch crate mirrors
  that path, which is the route for developing against unreleased
  frieze (with a CLI built from the same checkout).

### Workspaces

`cargo frieze generate` works from anywhere inside a workspace,
including virtual workspaces (a root `Cargo.toml` with no `[package]`
table). The frieze configuration itself stays per-package —
`[package.metadata.frieze]` lives in the member's own `Cargo.toml` —
and the workspace root may add one workspace-level key to name the
default target:

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["api-v1", "api-v2", "shared"]

[workspace.metadata.frieze]
package = "api-v1"   # the default target for `cargo frieze generate`
```

```console
$ cargo frieze generate                             # workspace root → the declared default (api-v1)
$ cargo frieze generate -p api-v2                   # explicit member, from any directory
$ cargo frieze generate -p api-v1 --output public   # flags compose: one member, one output
```

The target package is resolved in this order:

1. **`-p <name>` / `--package <name>`** — an explicit request always
   wins.
2. **The member directory you are inside** — running inside `api-v2/`
   targets `api-v2`, exactly like `cargo build`.
3. **The `[workspace.metadata.frieze] package` declaration** — at the
   workspace root (or anywhere outside a member directory), the
   declared default applies.
4. **The root package or sole member** — a workspace whose root is
   itself a package falls back to that package; a single-member
   workspace resolves to its sole member. A plain single-package
   crate therefore needs no configuration at all.

A virtual workspace with several members and no declaration fails with
an error listing the members and both selection mechanisms. Unknown
values are rejected the same way everything else is: `-p nope` and a
declaration naming a non-member list the actual members, and an
unknown key in the workspace table gets a "did you mean ...?"
suggestion.

Wherever the run starts, the declared `partial` / `output` paths
resolve relative to the **resolved member's** directory — never the
workspace root — and the scratch crate builds under the
workspace-level build directory (`target/frieze/<package>/`), seeded
with the workspace `Cargo.lock`, so the member's dependencies resolve
exactly as in your normal builds.

### CI: verify the committed documents with `--check`

When the generated documents are committed to the repository, CI
should fail whenever someone changes a schema type but forgets to
regenerate. `cargo frieze generate --check` runs the exact same
pipeline — including the build that collects the schemas — but writes
nothing: each output file is compared byte-for-byte against what a
normal run would write. Every output passing prints one
`up-to-date → <path>` line and exits 0; any stale or missing file is
named on stderr and the run exits 1. `--check` composes with `-p` and
`--output` the same way the write mode does.

A minimal GitHub Actions step:

```yaml
jobs:
  openapi-up-to-date:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install frieze-cli
      - run: cargo frieze generate --check
```

(Until the crates are published on crates.io, the install step must
point at a checkout instead, as in the quick start above.)

When the step fails, the fix is what the message says: run
`cargo frieze generate` locally and commit the refreshed documents.

## Optionality, in one paragraph

OpenAPI separates two concepts that Rust users often conflate:
**presence** (does the key appear in the object?) and **nullability**
(can the value be `null`?). `frieze` keeps them orthogonal — see the
[composite shapes table](docs/field-shapes.md#composite-shapes-presence-x-nullability)
for the full mapping. The short version: `Option<T>` alone is
required-and-nullable (matching serde's default behaviour),
`Option<T>` + `skip_serializing_if` is optional-and-non-nullable, and
`Maybe<T>` is the dedicated type for the remaining
"optional-and-nullable" combination that serde cannot express in a
single attribute.

## OpenAPI version

OAS 3.0 and 3.1 are both supported by every build — no Cargo feature
to pick. The version travels as per-document data: `from_schemas`
takes an explicit `frieze_openapi::Version`, a parsed `Document`
carries the version lifted from its `openapi:` field, and
serialization dispatches on it, so one program can emit a 3.0 document
and a 3.1 document side by side. The two versions encode nullability
differently (`nullable: true` vs `type: [..., "null"]`); see
[`docs/oas-versions.md`](docs/oas-versions.md) for the full encoding
table, the version-specific shapes for nullable references, and the
runtime `Version` handle.

## Auto-collection via `inventory`

`SchemasBuilder::new().from_inventory()` is available out of the box —
the `inventory` Cargo feature is on by default. Every non-generic
`#[derive(Schema)]` type is collected automatically, so a single call
is enough for the typical web-API server case:

```rust
let schemas = frieze::SchemasBuilder::new()
    .from_inventory()
    .build()?;
```

Generic types (`Page<T>`) are not auto-collected — Rust's `static`
cannot hold generic types, so the derive does not emit an inventory
entry for them. They are still registered transitively when a
non-generic root's field references the concrete instantiation
(`struct Foo { page: Page<Bar> }` walks into `Page<Bar>` from `Foo`),
so the manual `add` is only needed for *unreachable* generic
instances.

### When you still need explicit `add`

Two genuine cases require chaining `add::<T>()` after
`from_inventory()`:

1. **Documentation-only generic instantiations.** A generic instance
   like `Page<Bar>` that is not referenced by any non-generic root's
   field will not be reached by either channel. If you still want
   `Page<Bar>` in the OAS document (for example, to publish it as a
   standalone reusable component), register it as an isolated root
   with `add::<Page<Bar>>()`.

   ```rust
   let schemas = frieze::SchemasBuilder::new()
       .from_inventory()
       .add::<Page<Bar>>() // unreachable from any inventory-submitted root
       .build()?;
   ```

2. **Hand-written `impl Schema` for foreign types.** Types from
   external crates cannot carry `#[derive(Schema)]`, so they never
   submit to `inventory`. Provide hand-written `impl Schema` /
   `impl Register` / `impl IsRegistrable` blocks and register the type
   via `add::<ForeignType>()`.

`inventory` aggregates per binary, so every test in a given test
binary observes the same submission set. Tests that need an isolated
schemas set should reach for the explicit `add::<T>()` path.

### Opting out (no_std / WASM / embedded)

Targets that cannot host `inventory`'s linker-based registration can
opt out by disabling the default features:

```toml
frieze = { version = "...", default-features = false }
```

With the feature off, `SchemasBuilder::new().from_inventory()` is no
longer available and the derive macro's `inventory_submit!` expansion
becomes a no-op. Register schemas explicitly via `add::<T>()` instead.

## Documentation

| File                                                       | Topic                                            |
|------------------------------------------------------------|--------------------------------------------------|
| [`docs/field-shapes.md`](docs/field-shapes.md)             | Field types and presence/nullability             |
| [`docs/output-ordering.md`](docs/output-ordering.md)       | Output ordering guarantees                       |
| [`docs/oas-versions.md`](docs/oas-versions.md)             | OAS version handling and version differences     |
| [`RELEASING.md`](RELEASING.md)                             | Release procedure and version policy             |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
