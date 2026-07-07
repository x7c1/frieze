# Releasing

How a frieze release is versioned, verified, published to crates.io,
and checked afterwards.

## Version policy: one version, released in lockstep

Every crate in this workspace shares the single version declared under
`[workspace.package]` in the root `Cargo.toml`, and **every release
ships all nine crates together at that version**. This is not just a
convention — the CLI depends on it:

- `cargo-frieze` generates a scratch crate that pins
  `frieze = "=X.Y.Z"` and `frieze-usecase = "=X.Y.Z"`, where `X.Y.Z`
  is the CLI's own version. A `frieze` release without the matching
  `frieze-cli` (or vice versa) would leave that pin unresolvable.
- The collector rejects a target crate whose declared `frieze`
  version requirement cannot match that exact pin (see
  [Version skew](#what-the-version-skew-check-means-for-users)
  below), so mixed versions fail fast instead of silently collecting
  zero schemas.

Never publish a subset of the crates or bump a single crate's version
independently.

## The publishable set

All nine crates are published: `cargo install frieze-cli` must be able
to build the binary from crates.io alone, which pulls in the whole
internal graph, and the generated scratch crate additionally resolves
`frieze` / `frieze-usecase` as direct dependencies.

In dependency order (later crates depend on earlier ones):

| # | Crate | Depends on (internal) |
|---|----------------|--------------------------------------------|
| 1 | frieze-model | — |
| 2 | frieze-openapi | — |
| 3 | frieze-macros | — |
| 4 | frieze | frieze-model, frieze-macros |
| 5 | frieze-usecase | frieze-model, frieze-openapi |
| 6 | frieze-fs | frieze-model, frieze-openapi, frieze-usecase |
| 7 | frieze-cargo | frieze-model, frieze-openapi, frieze-usecase |
| 8 | frieze-wire | frieze-fs, frieze-cargo, frieze-usecase |
| 9 | frieze-cli | frieze-model, frieze-usecase, frieze-wire |

Internal dev-dependencies are declared as version-less path
dependencies, so `cargo publish` strips them and they play no role in
the release order.

## Cutting a release

1. **Bump the version** in the root `Cargo.toml`, in *two* places:
   - `[workspace.package] version`
   - the `version = "..."` of every internal crate entry under
     `[workspace.dependencies]` (these become the version requirements
     of the published manifests, so they must name the new version).
2. **Run the verification matrix** (all seven commands must pass;
   `cargo test --workspace` includes the end-to-end suite):

   ```console
   cargo fmt --all -- --check
   cargo build  --workspace
   cargo build  --workspace --no-default-features
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy --workspace --all-targets --no-default-features -- -D warnings
   cargo test   --workspace
   cargo test   --workspace --no-default-features
   ```

3. **Dry-run the publish** — packages, verifies, and orders all nine
   crates against a local overlay without uploading anything:

   ```console
   cargo publish --dry-run --workspace
   ```

4. **Land the bump on `main`** through the usual PR flow, then tag the
   merged commit:

   ```console
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. **Publish.** On cargo ≥ 1.90 a single command publishes the whole
   workspace in dependency order:

   ```console
   cargo publish --workspace
   ```

   With an older cargo, publish per crate in the table order above
   (`cargo publish -p <crate>`), waiting for each crate to become
   available before publishing its dependents.

## Post-publish verification

From a scratch directory **outside** this repository, so nothing
resolves against the checkout:

1. Install the CLI from crates.io:

   ```console
   cargo install frieze-cli --locked
   ```

2. Create a minimal package that depends on the released `frieze`
   (one `#[derive(Schema)]` type, one partial document, one
   `[[package.metadata.frieze.outputs]]` entry — the README's CLI
   quick start is exactly this) and run:

   ```console
   cargo frieze generate
   cargo frieze generate --check
   ```

   The first run must write the declared output; the second must
   report it `up-to-date` and exit 0.

### After the first release only

The README carries "until the crates are published on crates.io"
caveats (install from a checkout, path dependencies). Remove them once
the first publish has actually happened.

## What the version-skew check means for users

The scratch crate's `=X.Y.Z` pin means a user's crate and their
installed `cargo-frieze` must agree on the frieze version:

- If the crate's declared requirement (e.g. `frieze = "0.2"`) cannot
  match the installed CLI's version (e.g. `0.1.0`), the run fails
  before any build with an error naming both versions. The fix is to
  upgrade whichever side is behind: bump the `frieze` dependency, or
  `cargo install frieze-cli --version <matching>`.
- A crate that declares `frieze` as a **path dependency** into a
  checkout of this repository is exempt from the version check: the
  scratch crate mirrors the same path (and takes `frieze-usecase`
  from the same checkout), so both sides resolve one instance by
  construction. This is the local-development route — it assumes the
  `cargo-frieze` in use is built from that same checkout.

So a release announcement should remind users to update the CLI and
the library dependency together — after an upgrade of one side only,
the first `cargo frieze generate` (or `--check` in CI) fails with the
version-skew error rather than misbehaving quietly.
