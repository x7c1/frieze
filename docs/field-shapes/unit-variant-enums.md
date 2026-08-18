# Unit-variant enums

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

## Supported `rename_all` modes

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

## Composition with `Option`, `Vec`, and `Maybe`

Enum-typed fields obey the same composition rules as nested struct
fields — the `$ref` is wrapped by the same OAS-version-specific
nullable-reference shape. The mapping table from the
[nested struct nullability table](nested-structs.md#nullable-references-per-oas-version)
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

## Restrictions on enum shape

For a unit-variant enum, the macro additionally rejects:

- **Empty enums** (`enum Empty {}`) — OAS requires a non-empty
  `enum` array; an empty Rust enum has no inhabitants to enumerate.

Struct variants and tuple variants with multiple fields are rejected
in every mode — see [Internally-tagged enums](internally-tagged-enums.md)
below for the full table of accepted and rejected enum shapes.
