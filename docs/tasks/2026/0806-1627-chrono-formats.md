---
status: completed
pipeline_phase: null
plan: null
base_ref: null
perspectives: [rust-module-structure, consistency, completeness, minimalism, clarity]
retries_remaining: 1
check_command: "cargo fmt --all -- --check && cargo build --workspace && cargo build --workspace --no-default-features && cargo build -p frieze --features uuid1 && cargo build -p frieze --features chrono04 && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --no-default-features -- -D warnings && cargo clippy -p frieze --all-targets --features uuid1 -- -D warnings && cargo clippy -p frieze --all-targets --features chrono04 -- -D warnings && cargo test --workspace && cargo test --workspace --no-default-features && cargo test -p frieze --features uuid1 && cargo test -p frieze --features chrono04 && grep -q 'format: date-time' docs/field-shapes/README.md && grep -q chrono04 .github/workflows/ci.yml && grep -q chrono04 RELEASING.md"
assignee: null
branch: task/0806-1627-chrono-formats
created_at: 2026-08-06T16:27:42Z
updated_at: 2026-08-06T18:45:00Z
---

# feat: map chrono date types to `date-time` / `date` formats behind a `chrono04` feature

## Overview

Extend the string-format scalar pathway established by the `uuid1` feature
with a second opt-in cargo feature `chrono04` on the `frieze` crate
(optional dependency `chrono ^0.4`, `default-features = false` with its
`serde` feature NOT enabled by frieze):

- a `#[derive(Schema)]` struct field of type `chrono::DateTime<Tz>` (any
  `Tz: chrono::TimeZone`) produces the inline schema
  `{type: string, format: date-time}` — chrono's serde default serializes
  every `DateTime<Tz>` as an RFC 3339 string, so the declared format
  matches the wire shape for every time zone; the `Schema` impl is a
  blanket impl over `Tz` and `name()` returns `"DateTime"` regardless of
  the type argument,
- a field of type `chrono::NaiveDate` produces
  `{type: string, format: date}` (chrono's serde default is the ISO 8601
  date string, e.g. `2015-09-18`).

`chrono::NaiveDateTime` is **deliberately not supported** (no `Schema`
impl → the usual trait-bound compile error): it carries no offset, so its
serde output is not an RFC 3339 `date-time`, and mapping it anyway would
break the format-matches-wire guarantee. State this in
`docs/field-shapes/README.md`.

Follow the `uuid1` implementation end to end — it established the exact
pattern (see `docs/tasks/2026/0806-1218-uuid-format.md` and the resulting
code). Concretely:

- `frieze-model`: add `DateTime` and `Date` leaf variants to
  `PropertyType`, accept them in `ScalarSchema::new`, map `"DateTime"` /
  `"Date"` in `primitive_property_type_for`. Follow the compiler through
  every match site (no `_` arms).
- `frieze-usecase`: boundary arms
  `PropertyType::DateTime => (SchemaType::String, Some("date-time"), None)`
  and `PropertyType::Date => (SchemaType::String, Some("date"), None)`.
- `frieze`: feature-gated impls in
  `crates/libs/frieze/src/primitive_schema_impls.rs` — `NaiveDate` can
  reuse the existing macro; the `DateTime<Tz>` blanket impl needs a
  hand-written `impl<Tz: chrono::TimeZone> Schema for chrono::DateTime<Tz>`
  (+ matching `Register` no-op), still with **no** `IsRegistrable`.
  Declare `chrono` as an optional workspace dependency
  (`default-features = false`) and `chrono04 = ["dep:chrono"]` in
  `crates/libs/frieze/Cargo.toml`.
- `frieze-macros`: no changes (the `Reference` fall-through handles both
  types; `DateTime<Utc>` is a single-segment identifier with a generic
  argument, already accepted).
- Reservation is **unconditional**: `"DateTime"` and `"Date"` become
  reserved scalar names even with the feature off (same reasoning as
  `Uuid` — the boundary inline is not feature-gated). Update the
  `ReservedSchemaName` / `NonScalarPropertyType` error-message
  enumerations and the pinned wording tests in
  `crates/domain/frieze-model/tests/error_messages.rs`.
- Tests: feature-gated insta snapshot tests following the repository's
  established file convention — OAS-version-divergent cases split into
  `_oas_3_0.rs` / `_oas_3_1.rs` file pairs (see
  `derive_uuid_field_oas_3_0.rs` / `_oas_3_1.rs`). Cover a struct mixing
  `DateTime<Utc>`, `NaiveDate`, and a composition (`Option<DateTime<Utc>>`
  or `Vec<NaiveDate>`). Also add a non-feature-gated unit test proving a
  schema registered under the bare name `DateTime` (and `Date`) is
  rejected by `SchemasBuilder::build()` with the feature off, next to the
  existing reserved-name tests under
  `crates/libs/frieze/src/schemas_builder/`.
- Verification matrix lives in **four places** — `.github/workflows/ci.yml`,
  `CLAUDE.md`, `docs/oas-versions/README.md`, and `RELEASING.md` (including its
  "all N commands" count) — add the three `chrono04` build/clippy/test
  steps to all four, mirroring how the `uuid1` steps are placed.
- Docs: extend `docs/field-shapes/README.md` with the two mappings, the
  `NaiveDateTime` exclusion, and the same three user-setup facts the
  `uuid1` section carries (the user must add `chrono` to their own
  dependencies; the `04` suffix is the supported major — a different major
  fails with a trait-bound error; chrono's own `serde` feature is needed
  for structs deriving `Serialize`/`Deserialize`). Add the matching
  section to `README.md` and to the crate-level doc in
  `crates/libs/frieze/src/lib.rs` (docs.rs builds with default features,
  so the crate doc is where the feature is discoverable).

Work test-first: write the failing tests below before the implementation.

## Acceptance criteria

### Automated (pipeline-verified)

- [x] With `--features chrono04`, a derived struct with `DateTime<Utc>`
      and `NaiveDate` fields emits `{type: string, format: date-time}` and
      `{type: string, format: date}` inline, with at least one
      `Option`/`Vec` composition, pinned by feature-gated insta snapshot
      tests split per OAS version.
- [x] The `DateTime<Tz>` impl is generic over the time zone: a snapshot or
      unit test exercises a second `Tz` (e.g. `FixedOffset`) producing the
      identical schema.
- [x] Registering schemas under the bare names `DateTime` and `Date` is
      rejected by `SchemasBuilder::build()` with `Error::ReservedSchemaName`
      **without** the `chrono04` feature enabled (unit tests proving the
      reservation is unconditional).
- [x] The reserved-name enumerations in the error messages include
      `DateTime` and `Date`, with the pinned wording tests updated.
- [x] `docs/field-shapes/README.md` documents both mappings including the literal
      `format: date-time`, plus the `NaiveDateTime` exclusion (the grep
      gate in `check_command` enforces the literal).
- [x] `.github/workflows/ci.yml` and `RELEASING.md` gain the `chrono04`
      steps (grep gates in `check_command`), and the matrices in
      `CLAUDE.md` / `docs/oas-versions/README.md` match.

## Out of scope

- `NaiveDateTime`, `NaiveTime`, `Duration`, the `time` crate, `jiff`, and
  `Url` — no further scalar types beyond the two above.
- Any attribute-based format override mechanism.
- Changing the primitive-inline behaviour or the `uuid1` feature.
