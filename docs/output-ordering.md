# Output ordering

`frieze` guarantees specific output ordering even where the OAS treats
maps as unordered.

## Ordering guarantees

| Output                       | Order                                |
|------------------------------|--------------------------------------|
| `Schema.properties` keys     | Struct field declaration order       |
| `Schema.required` array      | Same order as `properties`           |
| `Schema.enum` array          | Variant declaration order            |
| `#/components/schemas` keys  | Alphabetical by schema name          |

`IndexMap` is used internally where insertion order matters; `BTreeMap`
where alphabetical order is desired.

## Canonical key order within a schema object

Within a single schema object, keys are emitted in canonical OAS
reading order:

```
$ref, type, items, format, minimum, allOf, oneOf, nullable (3.0 only),
properties, required
```

A schema object set to a `$ref` is emitted on its own — sibling keys
are dropped, matching the OAS rule that `$ref` schemas are treated as
leaves.

## Empty containers are omitted

Containers that would serialise as empty collections are omitted from
the output rather than emitted as `[]` or `{}`. The concrete rule today
covers `Schema.required`: when no fields are required, `required:` is
absent entirely (no `required: []` line).

This rule is enforced on **two independent paths** that must stay in
sync:

1. The `serde::Serialize` derive on `Schema` uses
   `#[serde(skip_serializing_if = "Vec::is_empty")]` on the `required`
   field, so any code path that relies on serde produces the right
   shape.
2. The custom YAML emitter (`schema_object_to_value`) does **not** go
   through serde at all — it walks the `Schema` and constructs YAML
   nodes manually. It re-applies the same empty-check before adding
   `required` to the output.

Both paths exist because frieze's primary YAML rendering bypasses
serde to control key ordering precisely (see [canonical key
order](#canonical-key-order-within-a-schema-object)). When changing
this rule, update both paths together — leaving one inconsistent will
make the output depend on which renderer was invoked.

Reason for the rule: `required: []` is technically valid OAS but
noisy and easy to misread as "no required fields known" vs. "this is
just an empty list"; omitting it removes the ambiguity.
