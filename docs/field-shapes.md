# Field shapes

`#[derive(Schema)]` recognises a fixed scalar set, optionally composed
with `Vec<T>`, `Option<T>`, and the frieze-defined `Maybe<T>` wrapper.
Field types that are themselves `Schema`-deriving structs are emitted as
`$ref` (see [Nested structs](#nested-structs)). A `Schema`-deriving
unit-variant enum is also a valid field type; it rides on the same
`$ref` transit path (see [Unit-variant enums](#unit-variant-enums)).

## Scalars

| Scalar Rust type | Maps to OAS                                            |
|------------------|--------------------------------------------------------|
| `i32`, `i64`     | `type: integer, format: int32 / int64`                 |
| `u32`, `u64`     | `type: integer, format: int32 / int64, minimum: 0`     |
| `f32`, `f64`     | `type: number, format: float / double`                 |
| `bool`           | `type: boolean`                                        |
| `String`         | `type: string`                                         |

`T` below stands for any of these scalars; `U` stands for another
`Schema`-deriving struct.

### Primitive `Schema` implementations

Primitive scalar types implement the `Schema` trait directly, with
schema names that follow the OAS type/format convention:

| Rust   | `<Type as Schema>::name()` |
|--------|----------------------------|
| `i32`  | `Int32`                    |
| `i64`  | `Int64`                    |
| `u32`  | `UInt32`                   |
| `u64`  | `UInt64`                   |
| `f32`  | `Float`                    |
| `f64`  | `Double`                   |
| `bool` | `Boolean`                  |
| `String` | `String`                 |

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

## Composite shapes (presence x nullability)

OpenAPI optionality has two **independent** axes: **presence** controls
whether the field name appears in the schema's `required` array, and
**nullability** controls whether the value may be `null`. The
combinations map to the following Rust shapes:

| Rust shape                                                            | Presence | Nullability                       |
|-----------------------------------------------------------------------|----------|-----------------------------------|
| `T`                                                                   | required | non-nullable                      |
| `Option<T>` (serde default)                                           | required | nullable                          |
| `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable                      |
| `Maybe<T>`                                                            | optional | nullable                          |
| `Vec<T>`                                                              | required | array, items as `T`               |
| `Vec<Option<T>>`                                                      | required | array, nullable items             |
| `Option<Vec<T>>`                                                      | required | nullable array                    |
| `Option<Vec<Option<T>>>`                                              | required | nullable array, nullable items    |
| `U` (another `Schema`-deriving struct)                                | required | `$ref` to `U`                     |
| `Option<U>` (serde default)                                           | required | nullable `$ref`                   |
| `Option<U>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable `$ref`               |
| `Maybe<U>`                                                            | optional | nullable `$ref`                   |
| `Vec<U>`                                                              | required | array of `$ref`                   |
| `Vec<Option<U>>`                                                      | required | array of nullable `$ref`          |

### Notes

- **`Option<T>` is required-and-nullable by default**, because serde
  emits `None` as `null` and expects the key to be present. This is
  surprising if you read `Option` as "may be omitted" — to get
  **optional + non-nullable**, pair `Option<T>` with the standard
  `#[serde(skip_serializing_if = "Option::is_none")]` attribute. The
  derive inspects that attribute and switches branches accordingly.
- **`Maybe<T>` is the dedicated three-state type** for "missing / null /
  present" — the one combination not expressible by `Option<T>` alone.
  Re-exported as `frieze::Maybe`. Add
  `#[serde(default, skip_serializing_if = "Maybe::is_missing")]` on the
  field to make missing-key handling work in both directions.
- **Nullability lives on the type tree** (`PropertyType::Nullable`),
  not on the property as a whole. That is how `Vec<Option<T>>` becomes
  an array of nullable items rather than a nullable array.

## Nested structs

A field whose type is another `Schema`-deriving struct (referred to as
`U` in the table above) is emitted as a `$ref` to
`#/components/schemas/<U::name()>`. The schema name is derived from the
Rust type name via the `Schema::name()` impl that `#[derive(Schema)]`
generates.

### Explicit transitive closure

Every reachable schema must be registered via `Schemas::add::<T>()`
on the same `SchemasBuilder`. The builder walks every property's type
tree and returns `Err(Error::UnresolvedReference(...))` for the first
`$ref` whose target schema is missing. Auto-discovery is intentionally
not provided — the registration list is the user's authoritative
inventory of what is exposed.

### Nullable references per OAS version

A sibling `nullable: true` cannot be attached to a `$ref` schema (OAS
3.0 ignores it; OAS 3.1 disallows it), so the renderer wraps nullable
references in a version-appropriate composition:

| Rust shape                                | OAS 3.0                                    | OAS 3.1                                       |
|-------------------------------------------|--------------------------------------------|-----------------------------------------------|
| `U`                                       | `{$ref: ...}`                              | `{$ref: ...}`                                 |
| `Option<U>` (serde default)               | `{allOf: [{$ref: ...}], nullable: true}`   | `{oneOf: [{$ref: ...}, {type: "null"}]}`      |
| `Maybe<U>`                                | `{allOf: [{$ref: ...}], nullable: true}`   | `{oneOf: [{$ref: ...}, {type: "null"}]}`      |
| `Vec<U>`                                  | `{type: array, items: {$ref: ...}}`        | `{type: array, items: {$ref: ...}}`           |
| `Vec<Option<U>>`                          | `items` carries the `allOf` shape          | `items` carries the `oneOf` shape             |

`Maybe<U>` requires the same serde attribute pair as `Maybe<T>` over
scalars: `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.

### Restrictions on field-position types

The macro rejects the following user-written forms as compile errors:

- **Qualified paths** (`mymod::User`) — bring the type into scope with
  a `use` statement first.

Generic arguments on the user type (`Foo<u32>`, `Page<User>`,
`Container<i64>`) are accepted; the field's `$ref` target is the
**composed schema name** of the instantiation (`Int32_Foo`, `User_Page`,
`Int64_Container`). See [Generic types](#generic-types) below for the
composition rule and registration requirements.

## Owned wrappers (`Box<T>`, `Rc<T>`, `Arc<T>`)

`std::boxed::Box`, `std::rc::Rc`, and `std::sync::Arc` are treated as
**transparent** owned wrappers with respect to the schema:

- `<Box<User> as Schema>::name()` returns `"User"`.
- `<Box<User> as Schema>::schema()` returns the same schema as
  `<User as Schema>::schema()`.
- The same delegation applies to `Rc<T>` and `Arc<T>`, and composes:
  `<Box<Box<User>> as Schema>::name() == "User"`.

This matches what serde produces on the wire: `Box<T>`, `Rc<T>`, and
`Arc<T>` all serialize as `T`'s wire form, so the schema must agree.

### Why transparency — recursive types

Recursive types in Rust require an indirection:

```rust
#[derive(Schema)]
struct Tree {
    value: i64,
    children: Vec<Box<Tree>>,   // Box<Tree> is required for sizedness
}
```

If `Box<Tree>` produced a separate schema entry instead of delegating
to `Tree`, every level of indirection would cascade into a new
synthetic schema name and the transitive-closure walk that resolves
`$ref` targets would never terminate. Transparent delegation gives a
self-referencing schema:

```yaml
Tree:
  type: object
  required: [value, children]
  properties:
    value: { type: integer, format: int64 }
    children:
      type: array
      items: { $ref: '#/components/schemas/Tree' }
```

`IsStructSchema` and `IsRegistrable` also propagate through the same
wrappers, so `Box<UserStruct>` is usable as the inner of an
internal-tagged enum variant and `Schemas::add::<Box<UserStruct>>()`
is equivalent to `Schemas::add::<UserStruct>()`.

### Scope: `Box` / `Rc` / `Arc` only

`Cell<T>`, `RefCell<T>`, `Mutex<T>`, and `RwLock<T>` are intentionally
**not** covered. They are interior-mutability primitives that rarely
appear in serialisable API shapes — a real REST handler typically
takes the lock, clones, then serializes, rather than serializing
through the lock guard. If the need arises later, the blanket impl
pattern in `frieze-usecase::wrapper_impls` is the template to follow.

## Generic types

`#[derive(Schema)]` accepts type parameters on the input struct and
emits an `impl Schema for Foo<T>` that requires `T: Schema` (the bound
is synthesised automatically, alongside the user's `where` clause).
The schema name and the schema body are both computed at
monomorphisation time: each specific instantiation
(`Page<User>`, `Container<i64>`, ...) is a separate entry under
`#/components/schemas`.

```rust
use frieze::Schema;

#[derive(Schema)]
struct Page<T> {
    items: Vec<T>,
    total: i64,
}

#[derive(Schema)]
struct User {
    id: i64,
    name: String,
}
```

```yaml
User:
  type: object
  required: [id, name]
  properties:
    id: { type: integer, format: int64 }
    name: { type: string }
User_Page:
  type: object
  required: [items, total]
  properties:
    items:
      type: array
      items: { $ref: '#/components/schemas/User' }
    total: { type: integer, format: int64 }
```

### Schema name composition

The name of a generic instantiation is the **suffix** form
`<Arg1>_<Arg2>_..._<BaseName>` — the type arguments come first in
declaration order, separated by `_`, with the base struct name last.
The composition is recursive: nested generic arguments expand into the
same flat sequence.

| Rust type                          | Composed schema name          |
|------------------------------------|-------------------------------|
| `Page<User>`                       | `User_Page`                   |
| `Container<i64>`                   | `Int64_Container`             |
| `Container<String>`                | `String_Container`            |
| `Pair<i32, f32>`                   | `Int32_Float_Pair`            |
| `Pair<i64, String>`                | `Int64_String_Pair`           |
| `Container<Container<i64>>`        | `Int64_Container_Container`   |

Primitive arguments contribute their `Schema::name()` (the OAS
type/format convention — `Int32`, `Int64`, `String`, ...). User
struct/enum arguments contribute their derived name.

The composition is intentionally flat and uses the same `_` separator
the OAS component-name pattern accepts. Collisions are possible in
principle (a 2-arg `Pair<A, B_C>` and a 3-arg `Triple<A, B, C>` with a
common base name could produce the same string); the
[duplicate-schema check](#explicit-transitive-closure) at
`Schemas::build()` reports them by name when they occur.

### Registration of generic instantiations

Each generic instantiation is a distinct schema entry and must be
registered on the builder explicitly:

```rust
frieze::schemas()
    .add::<Page<User>>()    // registers `User_Page`
    .add::<User>()          // registers `User`
    .build()?;
```

A struct that references a generic instantiation in a field (`profile:
Page<User>`) sees its `$ref` resolved through the standard
[transitive-closure walk](#explicit-transitive-closure); the builder
reports the missing target by its composed name:

```text
Err(UnresolvedReference(SchemaName("User_Page")))
```

### Primitive arguments are inlined, not referenced

Generic derive output cannot determine at expansion time whether a
type parameter is a primitive, so the inner field reference is always
emitted as `PropertyType::Reference(<T as Schema>::name())`. After
monomorphisation, a primitive `T` (e.g. `i64`) yields a reference
named after the primitive (`Int64`). Primitives implement `Schema` so
they can appear as generic arguments but **not** `IsRegistrable`, so
they cannot be added to `Schemas` and never appear under
`#/components/schemas`.

To keep this consistent, primitive references are **inlined as their
scalar shape at the leaf position** in the OAS output, and the
build-time reference walk treats primitive names as already resolved.
For `Container<i64>`:

```yaml
Int64_Container:
  type: object
  required: [value]
  properties:
    value: { type: integer, format: int64 }   # inlined, not $ref: Int64
```

No `components/schemas/Int64` entry is emitted, no
`Schemas::add::<i64>()` call is needed, and
`Schemas::add::<Container<i64>>().build()` succeeds standalone. The
same inline treatment applies to all eight primitive scalars (`Int32`,
`Int64`, `UInt32`, `UInt64`, `Float`, `Double`, `Boolean`, `String`).

### Owned-wrapper composition

`Box<T>`, `Rc<T>`, and `Arc<T>` are
[transparent owned wrappers](#owned-wrappers-boxt-rct-arct), so they do
**not** contribute to the composed name. `Box<User>`'s schema name is
`"User"`, not `"User_Box"`; `Vec<Box<Tree>>`'s element name is
`"Tree"`. This is what makes recursive type definitions
(`struct Tree { children: Vec<Box<Tree>> }`) emit a finite,
self-referencing schema instead of an unbounded `Tree_Box_Box_..."`
cascade.

### Recursive generic types

Recursive types compose naturally with generics. A `Node<T>` linked
list using `Option<Box<Node<T>>>` for the tail is self-referencing
through the same transparent-`Box` mechanism, so each instantiation
(`Node<User>`, `Node<i64>`, ...) is a single, finite schema entry.

```rust
#[derive(Schema)]
struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}
```

`Node<User>` registers as `User_Node` with `next` resolving back to
the same `User_Node` entry.

### Rejected generic shapes

- **Lifetime parameters** (`struct Borrowed<'a> { s: &'a str }`) —
  rejected at macro-expansion time. frieze schemas describe owned data
  layouts, and the OAS representation of a borrow is undefined.
- **Const generics** (`struct ArrN<const N: usize> { ... }`) —
  rejected at macro-expansion time. The OAS encoding of a
  compile-time constant in a schema name or shape is not in scope.
- **Trait objects as arguments** (`Box<dyn Schema>`) — rejected by
  rustc (the `T: Schema` bound is not satisfied by `dyn Schema`).
  frieze does not synthesise a curated diagnostic for this; the
  standard rustc message is sufficient.

## Unit-variant enums

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

### Supported `rename_all` modes

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

### Composition with `Option`, `Vec`, and `Maybe`

Enum-typed fields obey the same composition rules as nested struct
fields — the `$ref` is wrapped by the same OAS-version-specific
nullable-reference shape. The mapping table from the
[nested struct nullability table](#nullable-references-per-oas-version)
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

### Restrictions on enum shape

For a unit-variant enum, the macro additionally rejects:

- **Empty enums** (`enum Empty {}`) — OAS requires a non-empty
  `enum` array; an empty Rust enum has no inhabitants to enumerate.

Struct variants and tuple variants with multiple fields are rejected
in every mode — see [Internally-tagged enums](#internally-tagged-enums)
below for the full table of accepted and rejected enum shapes.

## Internally-tagged enums

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

### `discriminator.mapping` is deliberately omitted

The `discriminator` block emits `propertyName` only. The optional
`mapping` block is not emitted. If `mapping` pointed at each variant's
inner schema (e.g. `LoginData`), a strict reader would dispatch on the
tag value and then validate the payload against `LoginData` alone —
bypassing the `enum: [<wire_name>]` constraint that frieze synthesises
in the `allOf` arm. Omitting `mapping` makes readers shape-match
across the arms instead, so the tag-value constraint stays strict on
the wire. The shape is identical under both `oas-3-0` and `oas-3-1`.

### `rename` and `rename_all` on the tag value

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

### Per-variant doc comments

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

### Composition with `Option`, `Vec`, and `Maybe`

Internally-tagged enum-typed fields obey the same composition rules
as nested struct fields and string-enum fields — the `$ref` is
wrapped by the same OAS-version-specific nullable-reference shape.
The [nested struct nullability table](#nullable-references-per-oas-version)
applies unchanged when `U` is an internally-tagged enum.

### Accepted and rejected enum shapes

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

### Tag-vs-field collision is the user's responsibility

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

## Wire names (`rename` and `rename_all`)

frieze reads two `#[serde(...)]` attributes to compute each field's or
variant's **wire name** — the string that appears in the OAS schema
(`properties` key, `required` array entry, `enum` array value):

- `#[serde(rename = "literal")]` on a struct field or enum variant
  pins the wire name explicitly.
- `#[serde(rename_all = "<mode>")]` on a struct or enum container
  rewrites every field / variant identifier using one of the eight
  modes [listed above](#supported-rename_all-modes).

### Precedence

For each field or variant the wire name is computed as:

1. If the individual `#[serde(rename = "literal")]` is present, use
   the literal.
2. Otherwise, if the container has `#[serde(rename_all = "<mode>")]`,
   apply the mode to the Rust identifier.
3. Otherwise, the wire name is the Rust identifier verbatim.

This mirrors serde's own precedence. The wire name flows everywhere the
identifier used to — the `properties` map key, the `required` entries,
the `$ref`-side reference target name, and the per-variant bullet rows
inside an enum-level `description` (see
[Enum variant docs](#enum-variant-docs)).

The two `rename_all` rules (`apply_to_field` and `apply_to_variant` in
serde's terminology) differ — for instance `rename_all = "camelCase"`
on a struct produces `userId` from `user_id`, while on an enum it
produces `inactiveSince` from `InactiveSince`. frieze follows serde's
divergence so the generated schema matches what serde will emit on the
wire.

### Wire-name uniqueness

A struct cannot have two fields that map to the same wire name (after
`rename` / `rename_all` are applied), and an enum cannot have two
variants that map to the same value. Both are caught at macro-expansion
time with a diagnostic that names both sides of the collision and how
each side's name was produced. The check guards against serde's own
silent-acceptance behaviour: serde will compile a struct with two
fields renamed to the same wire name and then produce a schema that
loses one of them.

### Direction-split forms (unsupported)

`#[serde(rename(serialize = "...", deserialize = "..."))]` and the
matching `rename_all(serialize = ..., deserialize = ...)` form are
**rejected** as compile errors: a single OAS schema describes one shape
on the wire and cannot encode different names for serialize and
deserialize. The same constraint applies to `rename(serialize = "...")`
and `rename(deserialize = "...")` written alone. The symmetric
`#[serde(rename = "...")]` form is the supported way to pin a wire
name; if request and response shapes genuinely differ, split the type.

### Empty wire names

A wire name must be a non-empty string. `#[serde(rename = "")]` is a
compile error — both for explicit empty literals and for any case
where a `rename_all` rule would synthesise an empty result.

### Other `#[serde(...)]` attributes (unsupported)

The macro reads a small fixed allow-list (`rename`, `rename_all`,
`default`, `skip_serializing_if`) and rejects every other serde
attribute it understands, because each of them encodes a behaviour a
single OAS schema cannot faithfully represent:

| `#[serde(...)]`                       | Why frieze rejects it                                                                          |
|---------------------------------------|------------------------------------------------------------------------------------------------|
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

Attributes the macro doesn't recognise (e.g. serde's `crate = "..."`)
are passed through silently — they don't affect the generated schema.

## Compile-time validation of `Maybe<T>` fields

`Maybe<T>` only behaves correctly under serde when paired with the
attribute `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.
The `#[derive(Schema)]` macro enforces this: a `Maybe<T>` field without
both `default` **and** `skip_serializing_if = "Maybe::is_missing"` is a
compile error pointing at the offending field. This prevents schemas
from being silently inconsistent with their serialised form.

## Doc comments to `description`

Rust `///` doc comments on the four item kinds the macro can see
become the matching OAS `description`:

| Source                  | OAS placement                                                                |
|-------------------------|------------------------------------------------------------------------------|
| `///` on the struct     | `description` on the registered object schema                                |
| `///` on a struct field | `description` on that property's schema                                      |
| `///` on the enum       | `description` on the registered string-enum schema (top-level text)          |
| `///` on an enum variant| Composed into the enum's `description` as a `- <name>: <doc>` bullet row     |

### Normalisation

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

### Enum variant docs

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

### `$ref` field with description

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

## Unsupported shapes (compile error)

The macro rejects ambiguous or unsupported compositions before they
reach the schema-building code:

| Shape                | Reason                                                                                  |
|----------------------|-----------------------------------------------------------------------------------------|
| `Option<Option<T>>`  | serde flattens nested options.                                                          |
| `Vec<Vec<T>>`        | nested arrays are not supported.                                                        |
| `Vec<Maybe<T>>`      | array elements are always present on the wire; use `Vec<Option<T>>` for nullable items. |
| `Option<Maybe<T>>`   | presence is doubly defined.                                                             |
| `Maybe<Option<T>>`   | nullability is doubly defined.                                                          |
| `Maybe<Maybe<T>>`    | nested `Maybe` is not supported.                                                        |
