---
status: completed
pipeline_phase: null
plan: null
base_ref: null
perspectives: [rust-module-structure, consistency, completeness, minimalism, clarity]
retries_remaining: 1
check_command: "cargo fmt --all -- --check && cargo build --workspace && cargo build --workspace --no-default-features && cargo build -p frieze --features uuid1 && cargo build -p frieze --features chrono04 && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --no-default-features -- -D warnings && cargo clippy -p frieze --all-targets --features uuid1 -- -D warnings && cargo clippy -p frieze --all-targets --features chrono04 -- -D warnings && cargo test --workspace && cargo test --workspace --no-default-features && cargo test -p frieze --features uuid1 && cargo test -p frieze --features chrono04 && ! grep -q 'Tz: chrono::TimeZone' crates/libs/frieze/src/primitive_schema_impls.rs && ! grep -q 'escape hatch' docs/field-shapes/README.md"
assignee: null
branch: task/0807-0555-datetime-roundtrip-only
created_at: 2026-08-07T05:55:40Z
updated_at: 2026-08-07T07:42:00Z
---

# refactor: restrict the `DateTime<Tz>` schema impl to round-trippable time zones

## Overview

The `chrono04` feature currently implements `Schema` for `DateTime<Tz>` as
a blanket impl over every `Tz: chrono::TimeZone`. That is wider than the
wire supports: RFC 3339 carries only a numeric offset, so chrono can
serialize any `Tz` but implements `Deserialize` only for `Utc`, `Local`,
and `FixedOffset`. The blanket impl therefore lets a field like
`DateTime<chrono_tz::Tz>` obtain a schema while `#[derive(Deserialize)]`
on the same struct cannot compile, and the usual `deserialize_with`
escape hatch is itself rejected by the derive — a dead end that
`docs/field-shapes/README.md` currently documents as a caveat.

frieze exists to describe JSON wire types, and a named time zone is not a
JSON wire concept. Per the project's line-drawing principle (unsupported
shapes fail to compile rather than half-work), replace the blanket impl
with **three concrete impls** so the supported set is exactly the set
that round-trips the RFC 3339 wire:

- `crates/libs/frieze/src/primitive_schema_impls.rs`: delete the
  hand-written `impl<Tz: chrono::TimeZone> Schema for chrono::DateTime<Tz>`
  (and its `Register` twin) and instead implement for
  `chrono::DateTime<chrono::Utc>`, `chrono::DateTime<chrono::FixedOffset>`,
  and `chrono::DateTime<chrono::Local>` — the existing
  `impl_primitive_schema!` macro should accept these concrete types; all
  three keep the schema name `"DateTime"` and the
  `{type: string, format: date-time}` mapping. Any other `Tz`
  (e.g. `chrono_tz::Tz`) now fails at derive time with the ordinary
  missing-`Schema` trait-bound error, identical to every other
  unsupported type.
- Rationale to record in the module doc (concisely): the three zones are
  exactly the set chrono implements `Deserialize` for, so "has a frieze
  schema" now coincides with "round-trips the wire".
- Docs: rewrite the time-zone passage of the chrono section in
  `docs/field-shapes/README.md` — the Deserialize-dead-end caveat (including the
  `deserialize_with` sentence) is replaced by the simpler, stronger
  statement that other time zones do not implement `Schema` and fail to
  compile, and that named zones are not representable on the RFC 3339
  wire anyway (convert to `Utc` / `FixedOffset` at the boundary, or carry
  the zone name as a separate string field). Mirror the same statement
  briefly in `README.md`'s chrono section and the crate-level doc in
  `crates/libs/frieze/src/lib.rs` if they currently imply every `Tz` is
  accepted.
- Tests: the existing snapshot tests already cover `Utc` and
  `FixedOffset`; keep them passing unchanged (snapshot literals must not
  change). Update or replace the `DateTime<Tz>`-is-generic unit test
  (name/schema pinned per `Tz`) so it exercises the three supported
  zones. No new dependency for a compile-fail test: rejection relies on
  the standard trait-bound mechanism, same as all other unsupported
  types.
- While at it, verify (read the vendored crate sources under the cargo
  registry) that the remaining format scalars are serde-symmetric —
  `uuid::Uuid` and `chrono::NaiveDate` both implement `Serialize` and
  `Deserialize` — and note the finding in your final report; no code
  change expected from this check.

This is a behavior **narrowing** only: every previously-working
combination with `Utc` / `FixedOffset` / `Local` keeps working with
byte-identical output; combinations that previously half-worked
(schema without Deserialize) now fail to compile.

## Acceptance criteria

### Automated (pipeline-verified)

- [x] The blanket `impl<Tz: chrono::TimeZone>` is gone from
      `crates/libs/frieze/src/primitive_schema_impls.rs` (grep gate in
      `check_command`) and three concrete `DateTime` impls exist; all
      existing chrono snapshot tests pass with unchanged literals.
- [x] A unit test pins that `DateTime<Utc>`, `DateTime<FixedOffset>`, and
      `DateTime<Local>` all expose the schema name `"DateTime"` and the
      scalar `date-time` schema.
- [x] `docs/field-shapes/README.md` no longer contains the `deserialize_with`
      dead-end caveat (grep gate in `check_command`) and instead states
      that other time zones fail to compile.
- [x] The full 13-command matrix passes (both features on and off).

## Out of scope

- Splitting `primitive_schema_impls.rs` into a directory module.
- Any change to `NaiveDate`, `Uuid`, or the reserved-name machinery.
- RFC 9557 (`[America/New_York]` suffix) support.
