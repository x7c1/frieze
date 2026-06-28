# Output ordering

`frieze` guarantees specific output ordering even where the OAS treats
maps as unordered.

## Ordering guarantees

| Output                       | Order                                |
|------------------------------|--------------------------------------|
| Object schema `properties` keys | Struct field declaration order (each key is the field's wire name after `rename` / `rename_all`) |
| Object schema `required` array  | Same order as `properties`         |
| String-enum schema `enum` array | Variant declaration order (each value is the variant's wire name after `rename` / `rename_all`) |
| OneOf schema `oneOf` array      | Variant declaration order (each arm carries the variant's wire name as the synthesized tag constraint) |
| Variants in an enum-level `description` bullet list | Variant declaration order, using the same wire name as the `enum` array / `oneOf` arm |
| `#/components/schemas` keys     | Alphabetical by schema name        |

Renaming changes the **key text** in `properties` / `required` / `enum`
but never the order — declaration order is the single source of order
truth for fields and variants alike. The
[Wire names](field-shapes.md#wire-names-rename-and-rename_all) section
of `field-shapes.md` defines how each wire name is derived.

`IndexMap` is used internally where insertion order matters; `BTreeMap`
where alphabetical order is desired.

## Canonical key order within a schema object

A single canonical sequence is applied to every schema map emitted by
the custom YAML emitter; each schema kind shows the subset of keys
that apply.

```
$ref, type, description, format, minimum, items, required,
properties, allOf, oneOf, discriminator, nullable (3.0 only), enum
```

Per-kind visibility:

- **object schema**: `type, description, required, properties`
- **scalar property**: `type, description, format, minimum, nullable`
- **array property**: `type, description, items, nullable`
- **wrapper schema** (the schema that wraps a `$ref` in `allOf` /
  `oneOf`): `description, allOf, oneOf, nullable`
- **string-enum schema**: `type, description, enum`
- **oneOf schema** (internally-tagged enum): `description, oneOf, discriminator`

### Design rationale

- `type` stays at the start so a reader sees the schema kind first —
  it acts as a meaning locator that aligns with the OAS convention.
- `description` sits immediately after `type` because a long
  `properties` block can otherwise bury it.
- `required` is emitted before `properties` so the requiredness
  signal is not buried under what is usually the longest key. A
  reader sees "which keys are mandatory" before working through
  the property list.
- `format`, `minimum`, `items` are the type-specific elaboration
  keys; they come after `description` because they only refine the
  scalar / array shape.
- `allOf`, `oneOf`, `nullable` are composition / nullability tail
  keys; they trail the type and its description.
- `enum` is the string-enum tail key; the values list reads
  naturally at the end of the schema.

### `$ref` schemas

A schema whose `$ref` is set is a leaf. Sibling key handling differs
between OAS versions:

- **OAS 3.0**: `$ref` siblings are ignored on the wire, so the
  emitter emits the `$ref` alone. A sibling `description` is moved
  upstream into an `allOf` wrapper before reaching the emitter.
- **OAS 3.1**: a `description` is allowed as a sibling of `$ref`
  and is emitted next to it; other sibling fields are still dropped
  by the emitter and must be expressed through a wrapper schema
  produced upstream.

## Empty containers are omitted

Containers that would serialise as empty collections are omitted from
the output rather than emitted as `[]` or `{}`. The concrete rule today
covers an object schema's `required`: when no fields are required,
`required:` is absent entirely (no `required: []` line).

YAML rendering goes through the custom emitter
(`schema_object_to_value` in `frieze-usecase`), which walks the
sum-typed `SchemaObject` and constructs YAML nodes manually rather
than through `serde::Serialize`. The emitter applies the empty-check
before adding `required` to the output. When changing the omission
rule, update the emitter — there is no parallel serde path to keep in
sync.

Reason for the rule: `required: []` is technically valid OAS but
noisy and easy to misread as "no required fields known" vs. "this is
just an empty list"; omitting it removes the ambiguity.

## Example

```rust
/// A registered user of the system.
#[derive(Schema)]
struct User {
    /// The user's id.
    id: i64,
    /// The user's display name.
    name: String,
}
```

renders as:

```yaml
User:
  type: object
  description: A registered user of the system.
  required:
    - id
    - name
  properties:
    id:
      type: integer
      description: The user's id.
      format: int64
    name:
      type: string
      description: The user's display name.
```

`required` precedes `properties`; each property emits `type` then
its own `description` then any type-specific tail keys, in the same
order.
