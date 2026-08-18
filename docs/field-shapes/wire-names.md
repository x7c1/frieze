# Wire names (`rename` and `rename_all`)

frieze reads two `#[serde(...)]` attributes to compute each field's or
variant's **wire name** — the string that appears in the OAS schema
(`properties` key, `required` array entry, `enum` array value):

- `#[serde(rename = "literal")]` on a struct field or enum variant
  pins the wire name explicitly.
- `#[serde(rename_all = "<mode>")]` on a struct or enum container
  rewrites every field / variant identifier using one of the eight
  modes [listed above](unit-variant-enums.md#supported-rename_all-modes).

## Precedence

For each field or variant the wire name is computed as:

1. If the individual `#[serde(rename = "literal")]` is present, use
   the literal.
2. Otherwise, if the container has `#[serde(rename_all = "<mode>")]`,
   apply the mode to the Rust identifier.
3. Otherwise, the wire name is the Rust identifier verbatim.

This mirrors serde's own precedence. A field wire name flows to its
`properties` map key and `required` entry. A variant wire name flows to
its string-enum value or discriminator tag constraint and to the
per-variant bullet row inside an enum-level `description` (see
[Enum variant docs](descriptions.md#enum-variant-docs)).

`$ref` targets do not come from field wire names. They come from the
referenced field type's `Schema::name()`.

The two `rename_all` rules (`apply_to_field` and `apply_to_variant` in
serde's terminology) differ — for instance `rename_all = "camelCase"`
on a struct produces `userId` from `user_id`, while on an enum it
produces `inactiveSince` from `InactiveSince`. frieze follows serde's
divergence so the generated schema matches what serde will emit on the
wire.

## Wire-name uniqueness

A struct cannot have two fields that map to the same wire name (after
`rename` / `rename_all` are applied), and an enum cannot have two
variants that map to the same value. Both are caught at macro-expansion
time with a diagnostic that names both sides of the collision and how
each side's name was produced. The check guards against serde's own
silent-acceptance behaviour: serde will compile a struct with two
fields renamed to the same wire name and then produce a schema that
loses one of them.

## Direction-split forms (unsupported)

`#[serde(rename(serialize = "...", deserialize = "..."))]` and the
matching `rename_all(serialize = ..., deserialize = ...)` form are
**rejected** as compile errors: a single OAS schema describes one shape
on the wire and cannot encode different names for serialize and
deserialize. The same constraint applies to `rename(serialize = "...")`
and `rename(deserialize = "...")` written alone. The symmetric
`#[serde(rename = "...")]` form is the supported way to pin a wire
name; if request and response shapes genuinely differ, split the type.

## Empty wire names

A wire name must be a non-empty string. `#[serde(rename = "")]` is a
compile error — both for explicit empty literals and for any case
where a `rename_all` rule would synthesise an empty result.

## Other `#[serde(...)]` attributes (unsupported)

The macro reads a small fixed allow-list: field / variant `rename`,
container `rename_all`, enum-container `tag`, bare `default` on a
`Maybe<T>` field, and the exact `skip_serializing_if` predicates used by
`Option<T>` and `Maybe<T>`. It rejects every other serde attribute because
frieze cannot verify that a single generated OAS schema matches its wire
behaviour:

| `#[serde(...)]`                       | Why frieze rejects it                                                                          |
|---------------------------------------|------------------------------------------------------------------------------------------------|
| container `rename = "..."`           | Renaming the component schema and every `$ref` target is not yet modelled.                     |
| `alias = "..."`                       | Deserialize-only acceptance list; nothing on the OAS side accepts "additional names".          |
| `flatten`                             | Splices a sub-object's fields into the parent; the OAS schema would need synthetic flattening. |
| `content = "..."`                     | Adjacent tagging (`tag` + `content`) is not supported — use internal tagging without `content`. |
| `untagged`                            | Untagged enums are not supported — use internal tagging instead.                                |
| `transparent`                         | Container becomes its single field's wire shape; schema-side equivalent not yet modelled.      |
| `rename_all_fields = "..."`           | Per-variant rename rule; needs `oneOf` modelling.                                              |
| `skip` / `skip_serializing` / `skip_deserializing` | Excludes a field/variant from one or both directions; breaks request/response symmetry.        |
| `with = "..."` / `serialize_with` / `deserialize_with` | Replaces the (de)serialization with a custom path; frieze cannot infer the wire shape.   |
| `from = "..."` / `try_from = "..."` / `into = "..."` | Goes through a different type during (de)serialize; the wire shape is no longer the Rust type. |
| `other`                               | Catch-all variant for deserialize; no OAS counterpart.                                         |
| custom `default = "..."` or bare `default` outside `Maybe<T>` | Changes missing-field behaviour outside frieze's three-state mapping.          |
| any other `skip_serializing_if` predicate | Frieze cannot infer when the field disappears from an arbitrary predicate.                  |

Unrecognised entries (for example serde's `crate = "..."`) are rejected
as well. This fail-closed rule prevents a newly added or misspelled serde
attribute from silently making the schema diverge from the wire shape.
