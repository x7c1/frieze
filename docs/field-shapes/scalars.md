# Scalars

| Scalar Rust type       | Maps to OAS                                            |
|------------------------|--------------------------------------------------------|
| `i32`, `i64`           | `type: integer, format: int32 / int64`                 |
| `u32`, `u64`           | `type: integer, format: int32 / int64, minimum: 0`     |
| `f32`, `f64`           | `type: number, format: float / double`                 |
| `bool`                 | `type: boolean`                                        |
| `String`               | `type: string`                                         |
| `uuid::Uuid`           | `type: string, format: uuid` (`uuid1` feature)         |
| `chrono::DateTime<Tz>` | `type: string, format: date-time` (`chrono04` feature) |
| `chrono::NaiveDate`    | `type: string, format: date` (`chrono04` feature)      |

`Tz` is restricted to `Utc`, `FixedOffset` and `Local` — the time zones
that survive a round trip through the wire; see
[chrono date and time](#chrono-date-and-time-chrono04-feature).

## `uuid::Uuid` (`uuid1` feature)

`uuid::Uuid` is an opt-in scalar: enable the `uuid1` feature on the
`frieze` dependency to make it usable as a field type, and depend on
the `uuid` crate yourself. The feature only turns on frieze's impls
for the type — it does not put `uuid` into your crate's namespace, so
without your own `Cargo.toml` entry, `use uuid::Uuid;` does not
resolve.

```toml
frieze = { version = "...", features = ["uuid1"] }
uuid = { version = "1", features = ["serde"] }
```

The `1` in `uuid1` is the supported major version: `Uuid` from another
major (`uuid` 0.8, say) is a different type that does not implement
`Schema`, so such a field fails with a trait-bound error. The `serde`
feature belongs to `uuid`, not to frieze — frieze never serializes
your struct, but a struct that derives `Serialize` / `Deserialize`
around a `Uuid` field does not compile without it.

```rust
use frieze::Schema;
use uuid::Uuid;

#[derive(Schema)]
struct Session {
    id: Uuid,
}
```

```yaml
Session:
  type: object
  required: [id]
  properties:
    id: { type: string, format: uuid }
```

The field type must be written unqualified (`Uuid`, brought into scope
with `use uuid::Uuid;`) — the
[qualified-path restriction](nested-structs.md#restrictions-on-field-position-types)
applies here like it does to any other named type.

`format: uuid` is accurate because serde's default representation of
`uuid::Uuid` is the RFC 4122 hyphenated lowercase string, and frieze
rejects at compile time every serde attribute that would change a
field's wire representation (`with`, `serialize_with`, `into`, ...);
see
[Other `#[serde(...)]` attributes](wire-names.md#other-serde-attributes-unsupported).

`Option<Uuid>`, `Vec<Uuid>`, and `Maybe<Uuid>` follow the same
[composite-shape rules](composite-shapes.md) as
every other scalar. Because `Uuid` is a leaf scalar rather than a
`$ref`, the nullable forms are the plain scalar ones (`nullable: true`
under OAS 3.0, `type: [string, "null"]` under 3.1), not the
`allOf` / `oneOf` reference wraps.

The **name** `Uuid` is
[reserved](generics.md#primitive-scalar-names-are-reserved) whether or not the
feature is enabled.

## chrono date and time (`chrono04` feature)

`chrono::DateTime<Tz>` and `chrono::NaiveDate` are opt-in scalars:
enable the `chrono04` feature on the `frieze` dependency to make them
usable as field types, and depend on the `chrono` crate yourself. The
feature only turns on frieze's impls for those types — it does not put
`chrono` into your crate's namespace, so without your own `Cargo.toml`
entry, `use chrono::NaiveDate;` does not resolve.

```toml
frieze = { version = "...", features = ["chrono04"] }
chrono = { version = "0.4", features = ["serde"] }
```

The `04` in `chrono04` is chrono's `0.4` release series: chrono is
still pre-1.0, so `0.4` is the range Cargo treats as compatible, and
`chrono = "0.4"` is the entry to declare. A `DateTime` from another
series is a different type that does not implement `Schema`, so such a
field fails with a trait-bound error. The `serde` feature belongs to
`chrono`, not to frieze — frieze never serializes your struct, but a
struct that derives `Serialize` / `Deserialize` around a chrono field
does not compile without it.

The one chrono feature frieze does need is `clock`, which `chrono04`
turns on for the whole dependency graph: `chrono::Local` — and the
`Deserialize` impl for `DateTime<Local>` — lives behind it, so the
impls cannot cover all three supported time zones without it. Cargo
unifies features, so it reaches your build even if your own `chrono`
entry sets `default-features = false`, and it brings `iana-time-zone`
along.

```rust
use chrono::{DateTime, NaiveDate, Utc};
use frieze::Schema;

#[derive(Schema)]
struct Booking {
    created_at: DateTime<Utc>,
    checkout_on: NaiveDate,
}
```

```yaml
Booking:
  type: object
  required: [created_at, checkout_on]
  properties:
    created_at: { type: string, format: date-time }
    checkout_on: { type: string, format: date }
```

Both formats are accurate against chrono's serde defaults:
`DateTime<Tz>` writes an RFC 3339 timestamp and `NaiveDate` an ISO 8601
calendar date (`2015-09-18`), and frieze rejects at compile time every
serde attribute that would change a field's wire representation; see
[Other `#[serde(...)]` attributes](wire-names.md#other-serde-attributes-unsupported).

The time zone is part of the Rust type but not of the schema:
`DateTime<Utc>`, `DateTime<FixedOffset>`, and `DateTime<Local>` all
name the schema `DateTime` and all emit
`{type: string, format: date-time}`, because every one of them
serializes as an RFC 3339 string carrying its own offset.

Those three are also the **only** time zones with a `Schema` impl,
because they are the only ones chrono implements `Deserialize` for: a
schema exists exactly where the value round-trips. A field typed with
another `Tz` (`chrono_tz::Tz`, say) fails with the usual trait-bound
error, exactly like any other unsupported type. RFC 3339 carries a
numeric offset and no zone name, so a named zone is not representable
on the wire at all. Convert to `Utc` or `FixedOffset` at the boundary
of your API types, or carry the zone name as a separate `String` field.

The [qualified-path restriction](nested-structs.md#restrictions-on-field-position-types)
applies as usual, so write `DateTime<Utc>` rather than
`chrono::DateTime<chrono::Utc>`.

Both types compose with `Option`, `Vec`, and `Maybe` under the same
[composite-shape rules](composite-shapes.md) as
every other scalar. Because they are leaf scalars rather than `$ref`s,
the nullable forms are the plain scalar ones (`nullable: true` under
OAS 3.0, `type: [string, "null"]` under 3.1), not the `allOf` / `oneOf`
reference wraps.

The **names** `DateTime` and `Date` are
[reserved](generics.md#primitive-scalar-names-are-reserved) whether or not the
feature is enabled.

### `NaiveDateTime` is not supported

`chrono::NaiveDateTime` has **no** `Schema` impl, so a field of that
type fails with the usual trait-bound error. This is deliberate: a
naive timestamp carries no UTC offset, and serde writes it as
`2015-09-18T23:56:04` — which is not an RFC 3339 `date-time`. Mapping
it to `format: date-time` anyway would break the rule that the declared
format describes what actually goes on the wire. Use `DateTime<Utc>`
(or one of the other two supported time zones) when the value is a real
instant; if a naive timestamp genuinely is the wire format, model the
field as `String`.

## Primitive `Schema` implementations

Primitive scalar types implement the `Schema` trait directly, with
schema names that follow the OAS type/format convention:

| Rust                   | `<Type as Schema>::name()` |
|------------------------|----------------------------|
| `i32`                  | `Int32`                    |
| `i64`                  | `Int64`                    |
| `u32`                  | `UInt32`                   |
| `u64`                  | `UInt64`                   |
| `f32`                  | `Float`                    |
| `f64`                  | `Double`                   |
| `bool`                 | `Boolean`                  |
| `String`               | `String`                   |
| `uuid::Uuid`           | `Uuid`                     |
| `chrono::DateTime<Tz>` | `DateTime`                 |
| `chrono::NaiveDate`    | `Date`                     |

The primary purpose of these impls is to let primitives appear as
generic arguments — `Box<i64>`, `Page<String>`, etc. — so that derive
output for generic containers can use a uniform `T: Schema` trait
bound.

Primitives intentionally do **not** implement `IsRegistrable`, so
`Schemas::add::<i64>()` is rejected at compile time. The
`#[diagnostic::on_unimplemented]` message points users toward the fix
(wrap the scalar in a `#[derive(Schema)]` struct, or register the
containing type instead). The bare scalars are still useful as field
types and as generic arguments; they are not standalone
`#/components/schemas` entries.
