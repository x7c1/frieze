# `cargo frieze generate`

The `frieze-cli` crate ships the same pipeline as the library, exposed
as a cargo subcommand, so a crate can get its complete OAS document
without writing any generation code — no hand-written dump binary, no
`build.rs`.

## From zero to a generated document

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
`#[derive(Schema)]` on the types to expose, exactly as in
[Getting started](../README.md#getting-started). The installed CLI and
the declared `frieze` dependency must agree on the version — see the
version matching note below.

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

## Details worth knowing

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

## Workspaces

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

## CI: verify the committed documents with `--check`

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
point at a checkout instead, as in step 1 above.)

When the step fails, the fix is what the message says: run
`cargo frieze generate` locally and commit the refreshed documents.
