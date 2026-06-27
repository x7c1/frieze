# Output ordering

`frieze` guarantees specific output ordering even where the OAS treats
maps as unordered.

## Ordering guarantees

| Output                       | Order                                |
|------------------------------|--------------------------------------|
| Object schema `properties` keys | Struct field declaration order     |
| Object schema `required` array  | Same order as `properties`         |
| String-enum schema `enum` array | Variant declaration order (after `rename_all`) |
| `#/components/schemas` keys     | Alphabetical by schema name        |

`IndexMap` is used internally where insertion order matters; `BTreeMap`
where alphabetical order is desired.

## Canonical key order within a schema object

Within a single object schema, keys are emitted in canonical OAS
reading order:

```
$ref, type, items, format, minimum, allOf, oneOf, nullable (3.0 only),
properties, required
```

An object schema set to a `$ref` is emitted on its own — sibling keys
are dropped, matching the OAS rule that `$ref` schemas are treated as
leaves.

A string-enum schema emits two keys in this order:

```
type, enum
```

`type` is the literal string `string`; `enum` is the list of variant
values in source declaration order. Adding more top-level schema
kinds in the future appends new canonical key orders here.

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
