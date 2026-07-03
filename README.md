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
)
.expect("requested OAS version should match the compiled feature");
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
`type: [..., "null"]`) and are mutually exclusive. The version also
travels as data: `from_schemas` takes an explicit
`frieze_openapi::Version`, and a parsed `Document` carries the version
lifted from its `openapi:` field. For now the data-level version must
match the compiled feature. See
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
frieze = { version = "...", default-features = false, features = ["oas-3-0"] }
```

With the feature off, `SchemasBuilder::new().from_inventory()` is no
longer available and the derive macro's `inventory_submit!` expansion
becomes a no-op. Register schemas explicitly via `add::<T>()` instead.

## Documentation

| File                                                       | Topic                                            |
|------------------------------------------------------------|--------------------------------------------------|
| [`docs/field-shapes.md`](docs/field-shapes.md)             | Field types and presence/nullability             |
| [`docs/output-ordering.md`](docs/output-ordering.md)       | Output ordering guarantees                       |
| [`docs/oas-versions.md`](docs/oas-versions.md)             | OAS feature flags and version differences        |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
