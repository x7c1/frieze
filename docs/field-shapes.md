# Field shapes

`#[derive(Schema)]` recognises a fixed scalar set, optionally composed
with `Vec<T>`, `Option<T>`, and the frieze-defined `Maybe<T>` wrapper.
Field types that are themselves `Schema`-deriving structs are emitted as
`$ref` (see [Nested structs](#nested-structs)). A `Schema`-deriving
unit-variant enum is also a valid field type; it rides on the same
`$ref` transit path (see [Unit-variant enums](#unit-variant-enums)).

## Scalars

| Scalar Rust type | Maps to OAS                                            |
|------------------|--------------------------------------------------------|
| `i32`, `i64`     | `type: integer, format: int32 / int64`                 |
| `u32`, `u64`     | `type: integer, format: int32 / int64, minimum: 0`     |
| `f32`, `f64`     | `type: number, format: float / double`                 |
| `bool`           | `type: boolean`                                        |
| `String`         | `type: string`                                         |

`T` below stands for any of these scalars; `U` stands for another
`Schema`-deriving struct.

## Composite shapes (presence x nullability)

OpenAPI optionality has two **independent** axes: **presence** controls
whether the field name appears in the schema's `required` array, and
**nullability** controls whether the value may be `null`. The
combinations map to the following Rust shapes:

| Rust shape                                                            | Presence | Nullability                       |
|-----------------------------------------------------------------------|----------|-----------------------------------|
| `T`                                                                   | required | non-nullable                      |
| `Option<T>` (serde default)                                           | required | nullable                          |
| `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable                      |
| `Maybe<T>`                                                            | optional | nullable                          |
| `Vec<T>`                                                              | required | array, items as `T`               |
| `Vec<Option<T>>`                                                      | required | array, nullable items             |
| `Option<Vec<T>>`                                                      | required | nullable array                    |
| `Option<Vec<Option<T>>>`                                              | required | nullable array, nullable items    |
| `U` (another `Schema`-deriving struct)                                | required | `$ref` to `U`                     |
| `Option<U>` (serde default)                                           | required | nullable `$ref`                   |
| `Option<U>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable `$ref`               |
| `Maybe<U>`                                                            | optional | nullable `$ref`                   |
| `Vec<U>`                                                              | required | array of `$ref`                   |
| `Vec<Option<U>>`                                                      | required | array of nullable `$ref`          |

### Notes

- **`Option<T>` is required-and-nullable by default**, because serde
  emits `None` as `null` and expects the key to be present. This is
  surprising if you read `Option` as "may be omitted" — to get
  **optional + non-nullable**, pair `Option<T>` with the standard
  `#[serde(skip_serializing_if = "Option::is_none")]` attribute. The
  derive inspects that attribute and switches branches accordingly.
- **`Maybe<T>` is the dedicated three-state type** for "missing / null /
  present" — the one combination not expressible by `Option<T>` alone.
  Re-exported as `frieze::Maybe`. Add
  `#[serde(default, skip_serializing_if = "Maybe::is_missing")]` on the
  field to make missing-key handling work in both directions.
- **Nullability lives on the type tree** (`PropertyType::Nullable`),
  not on the property as a whole. That is how `Vec<Option<T>>` becomes
  an array of nullable items rather than a nullable array.

## Nested structs

A field whose type is another `Schema`-deriving struct (referred to as
`U` in the table above) is emitted as a `$ref` to
`#/components/schemas/<U::name()>`. The schema name is derived from the
Rust type name via the `Schema::name()` impl that `#[derive(Schema)]`
generates.

### Explicit transitive closure

Every reachable schema must be registered via `Schemas::add::<T>()`
on the same `SchemasBuilder`. The builder walks every property's type
tree and returns `Err(Error::UnresolvedReference(...))` for the first
`$ref` whose target schema is missing. Auto-discovery is intentionally
not provided — the registration list is the user's authoritative
inventory of what is exposed.

### Nullable references per OAS version

A sibling `nullable: true` cannot be attached to a `$ref` schema (OAS
3.0 ignores it; OAS 3.1 disallows it), so the renderer wraps nullable
references in a version-appropriate composition:

| Rust shape                                | OAS 3.0                                    | OAS 3.1                                       |
|-------------------------------------------|--------------------------------------------|-----------------------------------------------|
| `U`                                       | `{$ref: ...}`                              | `{$ref: ...}`                                 |
| `Option<U>` (serde default)               | `{allOf: [{$ref: ...}], nullable: true}`   | `{oneOf: [{$ref: ...}, {type: "null"}]}`      |
| `Maybe<U>`                                | `{allOf: [{$ref: ...}], nullable: true}`   | `{oneOf: [{$ref: ...}, {type: "null"}]}`      |
| `Vec<U>`                                  | `{type: array, items: {$ref: ...}}`        | `{type: array, items: {$ref: ...}}`           |
| `Vec<Option<U>>`                          | `items` carries the `allOf` shape          | `items` carries the `oneOf` shape             |

`Maybe<U>` requires the same serde attribute pair as `Maybe<T>` over
scalars: `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.

### Restrictions on field-position types

The macro rejects the following user-written forms as compile errors:

- **Qualified paths** (`mymod::User`) — bring the type into scope with
  a `use` statement first.
- **Generic arguments on user types** (`Foo<u32>`) — concrete user
  types only; generics over user schemas are not supported.

## Unit-variant enums

A Rust enum whose every variant is a unit variant derives an OAS
schema of the shape `type: string, enum: [...]`. The variant names
are emitted in source declaration order, after applying any
container-level `#[serde(rename_all = "...")]`. The schema is
registered under `#/components/schemas/<EnumName>` and is referenced
from any field that uses the enum as its type — the field carries a
`$ref` to the registered enum schema, the same transit path used for
nested struct references.

```rust
use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Active,
    InactiveSince,
}

#[derive(Schema, Serialize, Deserialize)]
struct User {
    id: i64,
    status: Status,
}
```

Both `Status` and `User` must be registered on the same
`SchemasBuilder`; the build resolves the `$ref` from `User.status`
to the registered `Status` schema.

### Supported `rename_all` modes

The values match serde's vocabulary:

- `lowercase`
- `UPPERCASE`
- `PascalCase`
- `camelCase`
- `snake_case`
- `SCREAMING_SNAKE_CASE`
- `kebab-case`
- `SCREAMING-KEBAB-CASE`

A value outside this list is rejected at compile time with a message
listing the accepted modes.

### Composition with `Option`, `Vec`, and `Maybe`

Enum-typed fields obey the same composition rules as nested struct
fields — the `$ref` is wrapped by the same OAS-version-specific
nullable-reference shape. The mapping table from the
[nested struct nullability table](#nullable-references-per-oas-version)
applies unchanged when `U` is an enum.

| Rust shape         | Emitted shape                                                                                  |
|--------------------|------------------------------------------------------------------------------------------------|
| `Status`           | `$ref` to the enum schema                                                                      |
| `Option<Status>`   | nullable reference (3.0: `allOf` + `nullable`; 3.1: `oneOf` + `{type: "null"}`)                 |
| `Maybe<Status>`    | same wrap as `Option<Status>`, plus optional presence                                          |
| `Vec<Status>`      | `type: array, items: {$ref}`                                                                   |
| `Vec<Option<Status>>` | `type: array`, items carry the nullable-reference wrap                                      |

`Maybe<Status>` requires the same serde attribute pair as
`Maybe<T>` over scalars:
`#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.

### Restrictions on enum shape

The macro rejects:

- **Tuple variants** (`Login(i64)`) — data-carrying variants need a
  richer OAS shape than a string enum and are not currently
  supported.
- **Struct variants** (`Login { user_id: i64 }`) — same reason.
- **Empty enums** (`enum Empty {}`) — OAS requires a non-empty
  `enum` array; an empty Rust enum has no inhabitants to enumerate.

## Wire names (`rename` and `rename_all`)

frieze reads two `#[serde(...)]` attributes to compute each field's or
variant's **wire name** — the string that appears in the OAS schema
(`properties` key, `required` array entry, `enum` array value):

- `#[serde(rename = "literal")]` on a struct field or enum variant
  pins the wire name explicitly.
- `#[serde(rename_all = "<mode>")]` on a struct or enum container
  rewrites every field / variant identifier using one of the eight
  modes [listed above](#supported-rename_all-modes).

### Precedence

For each field or variant the wire name is computed as:

1. If the individual `#[serde(rename = "literal")]` is present, use
   the literal.
2. Otherwise, if the container has `#[serde(rename_all = "<mode>")]`,
   apply the mode to the Rust identifier.
3. Otherwise, the wire name is the Rust identifier verbatim.

This mirrors serde's own precedence. The wire name flows everywhere the
identifier used to — the `properties` map key, the `required` entries,
the `$ref`-side reference target name, and the per-variant bullet rows
inside an enum-level `description` (see
[Enum variant docs](#enum-variant-docs)).

The two `rename_all` rules (`apply_to_field` and `apply_to_variant` in
serde's terminology) differ — for instance `rename_all = "camelCase"`
on a struct produces `userId` from `user_id`, while on an enum it
produces `inactiveSince` from `InactiveSince`. frieze follows serde's
divergence so the generated schema matches what serde will emit on the
wire.

### Wire-name uniqueness

A struct cannot have two fields that map to the same wire name (after
`rename` / `rename_all` are applied), and an enum cannot have two
variants that map to the same value. Both are caught at macro-expansion
time with a diagnostic that names both sides of the collision and how
each side's name was produced. The check guards against serde's own
silent-acceptance behaviour: serde will compile a struct with two
fields renamed to the same wire name and then produce a schema that
loses one of them.

### Direction-split forms (unsupported)

`#[serde(rename(serialize = "...", deserialize = "..."))]` and the
matching `rename_all(serialize = ..., deserialize = ...)` form are
**rejected** as compile errors: a single OAS schema describes one shape
on the wire and cannot encode different names for serialize and
deserialize. The same constraint applies to `rename(serialize = "...")`
and `rename(deserialize = "...")` written alone. The symmetric
`#[serde(rename = "...")]` form is the supported way to pin a wire
name; if request and response shapes genuinely differ, split the type.

### Empty wire names

A wire name must be a non-empty string. `#[serde(rename = "")]` is a
compile error — both for explicit empty literals and for any case
where a `rename_all` rule would synthesise an empty result.

### Other `#[serde(...)]` attributes (unsupported)

The macro reads a small fixed allow-list (`rename`, `rename_all`,
`default`, `skip_serializing_if`) and rejects every other serde
attribute it understands, because each of them encodes a behaviour a
single OAS schema cannot faithfully represent:

| `#[serde(...)]`                       | Why frieze rejects it                                                                          |
|---------------------------------------|------------------------------------------------------------------------------------------------|
| `alias = "..."`                       | Deserialize-only acceptance list; nothing on the OAS side accepts "additional names".          |
| `flatten`                             | Splices a sub-object's fields into the parent; the OAS schema would need synthetic flattening. |
| `tag = "..."`                         | Internally-tagged enums; needs `oneOf` + discriminator support that is not in Phase 1.         |
| `content = "..."`                     | Adjacent-tagged enums; same reason.                                                            |
| `untagged`                            | Untagged enums; same reason.                                                                   |
| `transparent`                         | Container becomes its single field's wire shape; schema-side equivalent not yet modelled.      |
| `rename_all_fields = "..."`           | Per-variant rename rule; needs `oneOf` modelling.                                              |
| `skip` / `skip_serializing` / `skip_deserializing` | Excludes a field/variant from one or both directions; breaks request/response symmetry.        |
| `with = "..."` / `serialize_with` / `deserialize_with` | Replaces the (de)serialization with a custom path; frieze cannot infer the wire shape.   |
| `from = "..."` / `try_from = "..."` / `into = "..."` | Goes through a different type during (de)serialize; the wire shape is no longer the Rust type. |
| `other`                               | Catch-all variant for deserialize; no OAS counterpart.                                         |

Attributes the macro doesn't recognise (e.g. serde's `crate = "..."`)
are passed through silently — they don't affect the generated schema.

## Compile-time validation of `Maybe<T>` fields

`Maybe<T>` only behaves correctly under serde when paired with the
attribute `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.
The `#[derive(Schema)]` macro enforces this: a `Maybe<T>` field without
both `default` **and** `skip_serializing_if = "Maybe::is_missing"` is a
compile error pointing at the offending field. This prevents schemas
from being silently inconsistent with their serialised form.

## Doc comments to `description`

Rust `///` doc comments on the four item kinds the macro can see
become the matching OAS `description`:

| Source                  | OAS placement                                                                |
|-------------------------|------------------------------------------------------------------------------|
| `///` on the struct     | `description` on the registered object schema                                |
| `///` on a struct field | `description` on that property's schema                                      |
| `///` on the enum       | `description` on the registered string-enum schema (top-level text)          |
| `///` on an enum variant| Composed into the enum's `description` as a `- <name>: <doc>` bullet row     |

### Normalisation

Each `#[doc = "..."]` attribute (which is what `///` expands to) is
read verbatim. Per line:

- One leading space is stripped if present (the rustdoc convention
  for the `///` form). Writing `///foo` with no space leaves the
  line unchanged.
- Trailing whitespace is trimmed.

Lines are joined with `\n`, and the final string has its trailing
blank lines stripped. If the result is empty (no `///` lines, or
every line is blank), the `description` key is omitted entirely —
the same empty-container omission rule that applies to `required`.

The description text is passed through to OAS unchanged; frieze does
not interpret or rewrite Markdown.

### Enum variant docs

OAS has no per-variant `description` slot (the `enum` array carries
plain strings), so the macro composes variant docs into the
enum-level `description`:

```rust
/// Lifecycle state of an entity.
#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    /// The entity is currently active.
    Active,
    /// The entity is no longer active.
    Inactive,
}
```

→

```yaml
Status:
  type: string
  description: |-
    Lifecycle state of an entity.

    - active: The entity is currently active.
    - inactive: The entity is no longer active.
  enum:
    - active
    - inactive
```

Composition rules:

- Variant names in the bullet list use the **OAS output name**
  (after `rename_all`) so they line up 1:1 with the `enum` array.
- A variant without a doc comment is omitted from the bullet list
  (a bare `- name:` row would be noise) but still appears in the
  `enum` array.
- When only the enum has a doc (no variant docs), only the
  enum-level text is emitted — no bullet list.
- When only variants have docs (no enum-level doc), only the
  bullet list is emitted.
- When neither is present, no `description` is emitted.

### `$ref` field with description

A `$ref` schema cannot carry sibling keys on the OAS 3.0 wire, so
when a `Reference`-typed field has its own doc-comment, the OAS
encoding differs between versions:

| Rust shape           | OAS 3.0                                                       | OAS 3.1                                              |
|----------------------|---------------------------------------------------------------|------------------------------------------------------|
| `U` (no doc)         | `{$ref}`                                                      | `{$ref}`                                             |
| `U` + `///`          | `{description, allOf: [{$ref}]}`                              | `{$ref, description}`                                |
| `Option<U>` (no doc) | `{allOf: [{$ref}], nullable: true}`                           | `{oneOf: [{$ref}, {type: "null"}]}`                  |
| `Option<U>` + `///`  | `{description, allOf: [{$ref}], nullable: true}`              | `{description, oneOf: [{$ref}, {type: "null"}]}`     |

The description always rides on the **outer** wrapper, never inside
the `allOf` / `oneOf` array.

## Unsupported shapes (compile error)

The macro rejects ambiguous or unsupported compositions before they
reach the schema-building code:

| Shape                | Reason                                                                                  |
|----------------------|-----------------------------------------------------------------------------------------|
| `Option<Option<T>>`  | serde flattens nested options.                                                          |
| `Vec<Vec<T>>`        | nested arrays are not supported.                                                        |
| `Vec<Maybe<T>>`      | array elements are always present on the wire; use `Vec<Option<T>>` for nullable items. |
| `Option<Maybe<T>>`   | presence is doubly defined.                                                             |
| `Maybe<Option<T>>`   | nullability is doubly defined.                                                          |
| `Maybe<Maybe<T>>`    | nested `Maybe` is not supported.                                                        |
