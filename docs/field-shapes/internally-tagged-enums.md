# Internally-tagged enums

A Rust enum whose every variant is a **newtype wrapping a
`Schema`-implementing struct** and that carries
`#[serde(tag = "<discriminator>")]` derives an OAS `oneOf` schema with
a top-level `discriminator` block. This is the single supported form
for data-carrying enums.

```rust
use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct LoginData { user_id: i64, session: String }

#[derive(Schema, Serialize, Deserialize)]
struct LogoutData { reason: String }

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Event {
    Login(LoginData),
    Logout(LogoutData),
}
```

Renders as:

```yaml
Event:
  oneOf:
    - allOf:
        - $ref: '#/components/schemas/LoginData'
        - type: object
          required: [kind]
          properties:
            kind:
              type: string
              enum: [Login]
    - allOf:
        - $ref: '#/components/schemas/LogoutData'
        - type: object
          required: [kind]
          properties:
            kind:
              type: string
              enum: [Logout]
  discriminator:
    propertyName: kind
```

Each arm composes an `allOf` of (1) a `$ref` to the inner struct's
schema and (2) a synthetic object constraining the discriminator
property to the variant's wire name. The two-step `allOf` keeps the
inner struct schema reusable elsewhere (a flat inline-merged shape
would prevent it).

## `discriminator.mapping` is deliberately omitted

The `discriminator` block emits `propertyName` only. The optional
`mapping` block is not emitted. If `mapping` pointed at each variant's
inner schema (e.g. `LoginData`), a strict reader would dispatch on the
tag value and then validate the payload against `LoginData` alone —
bypassing the `enum: [<wire_name>]` constraint that frieze synthesises
in the `allOf` arm. Omitting `mapping` makes readers shape-match
across the arms instead, so the tag-value constraint stays strict on
the wire. The shape is identical under both OAS 3.0 and 3.1.

## `rename` and `rename_all` on the tag value

Wire-name precedence on variants is the same rule used everywhere
else in frieze:

1. an individual `#[serde(rename = "literal")]` on the variant pins
   the tag value;
2. otherwise, the container's `#[serde(rename_all = "<mode>")]` is
   applied to the variant identifier;
3. otherwise, the Rust identifier is used verbatim.

All variant wire names must be pairwise distinct (the same uniqueness
check that guards struct field wire names and unit-enum variant wire
names).

## Per-variant doc comments

OAS has no per-variant `description` slot in `oneOf`. The macro
composes `///` doc comments on the variants into the enclosing
schema's `description` as a bullet list, exactly as it does for
unit-variant enums:

```rust
/// A user session event.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Event {
    /// The user logged in.
    Login(LoginData),
    /// The user logged out.
    Logout(LogoutData),
}
```

→ the `Event` schema's `description` reads:

```
A user session event.

- Login: The user logged in.
- Logout: The user logged out.
```

Bullet names use the wire name (post `rename_all` / per-variant
`rename`) so they line up 1:1 with the `oneOf` arms' tag values.

## Composition with `Option`, `Vec`, and `Maybe`

Internally-tagged enum-typed fields obey the same composition rules
as nested struct fields and string-enum fields — the `$ref` is
wrapped by the same OAS-version-specific nullable-reference shape.
The [nested struct nullability table](nested-structs.md#nullable-references-per-oas-version)
applies unchanged when `U` is an internally-tagged enum.

## Accepted and rejected enum shapes

| Shape                                                                     | Status                                                                  |
|---------------------------------------------------------------------------|-------------------------------------------------------------------------|
| Unit-only enum (no tag)                                                   | string-enum schema                                                      |
| Internally-tagged enum, every variant a newtype-of-Schema-struct          | `oneOf` schema with `discriminator.propertyName`                        |
| Data-carrying variants without `#[serde(tag = "...")]`                    | compile error — `tag` attribute is required                             |
| `#[serde(tag = "...")]` mixed with a unit variant                         | compile error — every variant must be a newtype-of-struct               |
| Newtype inner is a primitive (`String`, `i64`, etc.)                      | compile error — inner must be a struct that implements `Schema`         |
| Newtype inner is `Vec<T>` / `Option<T>` / `Maybe<T>`                      | compile error — inner must be a struct that implements `Schema`         |
| Newtype inner is itself a Schema-deriving enum (string-enum / `oneOf`)    | compile error via the `IsStructSchema` bound (rustc surfaces the diagnostic message) |
| Struct variants (`Login { user_id: i64 }`)                                | compile error in every mode                                             |
| Tuple variants with multiple fields (`Point(i32, i32)`)                   | compile error in every mode                                             |
| `#[serde(untagged)]`                                                      | compile error                                                           |
| `#[serde(tag = "...", content = "...")]` (adjacent tagging)               | compile error                                                           |
| Unit-only enum with an explicit `#[serde(tag = "...")]`                   | compile error — drop the attribute to emit a string-enum schema         |
| Empty enum (`enum Empty {}`)                                              | compile error — no inhabitants to enumerate                             |

## Tag-vs-field collision is the user's responsibility

If the tag name (`#[serde(tag = "kind")]`) collides with an existing
field of the inner struct (`struct LoginData { kind: String, ... }`),
the wire shape silently breaks at the serde layer and the emitted OAS
schema carries a contradictory pair of constraints on the same
property (`type: string` from the inner schema vs `const: <wire_name>`
from the synthesised tag arm). frieze does not check for this:

- a pure compile-time check requires cross-derive coordination;
- a runtime check at `Schemas::build()` time has weak fail-fast value;
- mainstream OAS validators catch the resulting contradictory schema.

The expected discipline is to choose a tag name that does not collide
with any field of any of the inner structs in the enum. The
"data-carrying variants without `#[serde(tag = "...")]`" error message
names the typical safe choices (`type`, `kind`, `label`, `event_type`).
