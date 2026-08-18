---
status: completed
pipeline_phase: null
plan: null
base_ref: null
retries_remaining: 1
check_command: "cargo fmt --all -- --check && cargo build --workspace && cargo build --workspace --no-default-features && cargo build -p frieze --features uuid1 && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --no-default-features -- -D warnings && cargo clippy -p frieze --all-targets --features uuid1 -- -D warnings && cargo test --workspace && cargo test --workspace --no-default-features && cargo test -p frieze --features uuid1 && grep -q 'format: uuid' docs/field-shapes/README.md && grep -q uuid1 .github/workflows/ci.yml"
assignee: null
branch: task/0806-1218-uuid-format
created_at: 2026-08-06T12:18:13Z
updated_at: 2026-08-06T13:52:00Z
---

# feat: map `uuid::Uuid` fields to `type: string, format: uuid` behind a `uuid1` feature

## Overview

Add the first string-format scalar to the derive: with a new opt-in cargo
feature `uuid1` on the `frieze` crate (optional dependency `uuid ^1`), a
`#[derive(Schema)]` struct field of type `uuid::Uuid` produces the inline
schema `{type: string, format: uuid}`. The serde default representation of
`uuid::Uuid` is the RFC 4122 hyphenated lowercase string, so the declared
format matches the actual wire shape; representation-changing serde
attributes are already rejected by the derive, which keeps that guarantee.

Follow the existing primitive-scalar pathway end to end — `Uuid` is "the
ninth primitive". Concretely:

- `frieze-model`: add a `Uuid` leaf variant to `PropertyType`
  (`crates/domain/frieze-model/src/property_type.rs`), accept it in
  `ScalarSchema::new` (`crates/domain/frieze-model/src/scalar_schema.rs`),
  and map the name `"Uuid"` in `primitive_property_type_for`. Adding the
  variant makes rustc surface every match site that needs a new arm (the
  codebase bans `_` arms on frieze enums) — follow the compiler.
- `frieze-usecase`: in `property_type_to_object_schema`
  (`crates/domain/frieze-usecase/src/boundary.rs`), map
  `PropertyType::Uuid` to `(SchemaType::String, Some("uuid"), None)`.
- `frieze`: feature-gated `Schema` + `Register` impls for `uuid::Uuid`
  (name `"Uuid"`, schema `new_scalar(PropertyType::Uuid)`, `Register` a
  no-op, **no** `IsRegistrable`), following the shape of
  `crates/libs/frieze/src/primitive_schema_impls.rs`. Declare `uuid` as an
  optional workspace dependency and `uuid1 = ["dep:uuid"]` in
  `crates/libs/frieze/Cargo.toml`.
- `frieze-macros`: no changes — the derive already routes unknown
  identifiers through `PropertyType::Reference(<T as Schema>::name())`, and
  the boundary inlines references whose name is a known primitive. Do not
  add `"Uuid"` to the macro's textual scalar match.
- Reservation is **unconditional** (not feature-gated): once `"Uuid"` is in
  `primitive_property_type_for`, the boundary inlines any `Reference("Uuid")`
  regardless of the feature, so a user type registered under the bare name
  `Uuid` must be rejected by the existing reserved-name check even when
  `uuid1` is off — otherwise the silent-hijack hazard the check exists for
  would reopen. Update the `ReservedSchemaName` / `NonScalarPropertyType`
  error messages that enumerate the reserved scalar names to include `Uuid`,
  and update the pinned wording tests in
  `crates/domain/frieze-model/tests/error_messages.rs` in the same change.
- While touching `primitive_property_type_for`, add the reserved-name check
  in `SchemasBuilder::push_unique` to the "single source of truth used by:"
  list in its doc comment — a known omission from the change that introduced
  the check.
- CI (`.github/workflows/ci.yml`): add build/clippy/test steps for the
  `uuid1` feature path (mirroring the existing no-default-features steps),
  and update the build/test matrix documented in `CLAUDE.md` and, if it
  describes the matrix, `docs/oas-versions/README.md`.
- Docs: extend `docs/field-shapes/README.md` with the `uuid::Uuid` field shape
  (the emitted schema, the wire guarantee, composition with
  `Option`/`Vec`/`Maybe`, and the feature flag). Check `README.md` for any
  user-visible surface that should mention the feature.

Work test-first: write the failing tests below before the implementation.

## Acceptance criteria

### Automated (pipeline-verified)

- [x] With `--features uuid1`, a derived struct with a `uuid::Uuid` field
      emits `{type: string, format: uuid}` inline (feature-gated insta
      snapshot test in the `frieze` crate, alongside the existing derive
      tests), and compositions `Option<Uuid>` (nullable) and `Vec<Uuid>`
      (array items) render through the existing wrapping rules in the same
      snapshot(s).
- [x] Registering a schema under the bare name `Uuid` is rejected by
      `SchemasBuilder::build()` with `Error::ReservedSchemaName` **without**
      the `uuid1` feature enabled (unit test beside the existing
      reserved-name tests, proving the reservation is unconditional).
- [x] The reserved-name enumerations in the error messages include `Uuid`,
      and the pinned wording tests in
      `crates/domain/frieze-model/tests/error_messages.rs` are updated to
      match.
- [x] `docs/field-shapes/README.md` documents the `uuid::Uuid` mapping including
      the literal `format: uuid` (the grep gate in `check_command` enforces
      it).
- [x] `.github/workflows/ci.yml` gains `uuid1` feature steps (the grep gate
      in `check_command` enforces it), and the matrix documented in
      `CLAUDE.md` matches.

## Out of scope

- `chrono` types (`DateTime<Tz>` / `NaiveDate`) and any other string-format
  scalar — this task establishes the feature-gated pathway with the single
  simplest type.
- Any attribute-based format override mechanism.
- Changing how primitive references are inlined at the boundary.
