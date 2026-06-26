# frieze

Generate OpenAPI Schema Objects from Rust types via `#[derive(Schema)]`.

> Status: early Phase 1. Public API may shift.

## Quick start

`frieze` derives a schema description from a plain Rust struct and lets
you render it as YAML.

```rust
use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct User {
    // Required + non-nullable.
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

let schemas = frieze::schemas()
    .add::<User>()
    .build()
    .expect("schemas build should succeed");
println!("{}", frieze::to_yaml(&schemas));
```

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

Pick exactly one of `oas-3-0` (default) or `oas-3-1` as a Cargo feature.
The two encode nullability differently (`nullable: true` vs
`type: [..., "null"]`) and are mutually exclusive. See
[`docs/oas-versions.md`](docs/oas-versions.md) for the full encoding
table and the version-specific shapes for nullable references.

## Documentation

| File                                                       | Topic                                            |
|------------------------------------------------------------|--------------------------------------------------|
| [`docs/field-shapes.md`](docs/field-shapes.md)             | Field types and presence/nullability             |
| [`docs/output-ordering.md`](docs/output-ordering.md)       | Output ordering guarantees                       |
| [`docs/oas-versions.md`](docs/oas-versions.md)             | OAS feature flags and version differences        |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
