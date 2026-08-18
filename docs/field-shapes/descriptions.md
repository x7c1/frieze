# Doc comments to `description`

Rust `///` doc comments on the four item kinds the macro can see
become the matching OAS `description`:

| Source                  | OAS placement                                                                |
|-------------------------|------------------------------------------------------------------------------|
| `///` on the struct     | `description` on the registered object schema                                |
| `///` on a struct field | `description` on that property's schema                                      |
| `///` on the enum       | `description` on the registered string-enum schema (top-level text)          |
| `///` on an enum variant| Composed into the enum's `description` as a `- <name>: <doc>` bullet row     |

## Normalisation

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

## Enum variant docs

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

## `$ref` field with description

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
