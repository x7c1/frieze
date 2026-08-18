---
status: completed
pipeline_phase: null
plan: null
base_ref: null
retries_remaining: 1
check_command: "cargo fmt --all -- --check && cargo build --workspace && cargo build --workspace --no-default-features && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --no-default-features -- -D warnings && cargo test --workspace && cargo test --workspace --no-default-features && grep -q ReservedSchemaName docs/field-shapes/README.md"
assignee: null
branch: task/0806-1030-reserved-schema-names
created_at: 2026-08-06T10:30:46Z
updated_at: 2026-08-06T12:14:00Z
---

# feat: reject registering schemas under reserved scalar names

## Overview

The boundary conversion inlines any `PropertyType::Reference` whose name
matches one of the eight reserved primitive scalar names — `Int32`, `Int64`,
`UInt32`, `UInt64`, `Float`, `Double`, `Boolean`, `String` — as the matching
scalar shape instead of emitting a `$ref` (see
`primitive_property_type_for` in `crates/domain/frieze-model/src/property_type.rs`
and `property_type_to_object_schema` in
`crates/domain/frieze-usecase/src/boundary.rs`; the behaviour is documented in
`docs/field-shapes/README.md` around the "primitive references are inlined" section).

Consequence: if a user defines their own `#[derive(Schema)] pub struct Int64`
(or any type whose registered schema name exactly equals a reserved name),
every reference to it is silently hijacked and rendered as the primitive
scalar shape, while the registered schema sits unreferenced under
`components/schemas`. The existing `Error::SchemaConflict` cannot catch this:
primitive scalars are never registered under `#/components/schemas`, so no
name conflict ever materialises. The silent breakage must become an explicit
error.

Make `SchemasBuilder` (`crates/libs/frieze/src/schemas_builder.rs`) detect a
registration whose full name exactly matches a reserved scalar name, and fail
at `build()` with a new `frieze-model` error variant (suggested:
`Error::ReservedSchemaName { name: SchemaName }`), following the deferred
first-occurrence-wins pattern already used for `Error::SchemaConflict`.
Requirements:

- Hook the detection in `push_unique`, the funnel all registration paths go
  through (`add::<T>()`, transitive `register_into`, `from_inventory`), so
  every path is covered.
- Use `primitive_property_type_for(name).is_some()` as the single source of
  truth for "is reserved" — do not duplicate the name list.
- Report the reserved name before the conflict / unresolved-reference checks
  in `build()` — it is the most fundamental misuse.
- Only exact matches are rejected. Composed generic names (`Int64_Container`)
  and namespaced names (`v1.Int64` — namespaced names always contain a dot,
  so they can never equal a reserved bare name) must remain accepted.
- The error message must name the offending schema and offer the two
  remedies, mirroring the wording approach of `SchemaConflict`'s message:
  rename the Rust type, or place it under a `#[frieze(namespace)]` module so
  the registered name carries a prefix.
- Work test-first: write the failing tests below before the implementation.

## Acceptance criteria

### Automated (pipeline-verified)

- [x] Registering a schema named exactly `Int64` makes `SchemasBuilder::build()`
      return the new reserved-name error — unit test beside the existing
      `SchemasBuilder` tests, constructing the schema via `frieze-model`
      constructors as the neighbouring tests do.
- [x] Registering schemas named `Int64_Container` and `v1.Int64` still
      builds successfully (unit tests proving the exact-match-only rule).
- [x] The error's `Display` message names the offending schema and contains
      both remedies (rename / `#[frieze(namespace)]`), pinned by a unit test
      following the placement of the existing error-message pin tests.
- [x] `docs/field-shapes/README.md` documents the reserved-name rejection next to
      the primitive-inline section, mentioning `ReservedSchemaName`
      (`grep -q ReservedSchemaName docs/field-shapes/README.md` is appended to
      `check_command`, so the check phase enforces it).

## Out of scope

- Adding new reserved names or new primitive scalar types.
- Changing the primitive-inline behaviour in the boundary conversion itself.
- Any attribute-based rename mechanism (`#[serde(rename)]` on containers).
