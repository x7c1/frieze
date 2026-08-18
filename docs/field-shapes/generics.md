# Generic types

`#[derive(Schema)]` accepts type parameters on the input struct and
emits an `impl Schema for Foo<T>` that requires `T: Schema`, plus an
`impl Register for Foo<T>` that requires `T: Register` (the bounds are
synthesised automatically, alongside the user's `where` clause).
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

## Schema name composition

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
[duplicate-schema check](nested-structs.md#explicit-transitive-closure) at
`Schemas::build()` reports them by name when they occur.

## Registration of generic instantiations

Each generic instantiation is a distinct schema entry and must be
registered on the builder explicitly:

```rust
frieze::SchemasBuilder::new()
    .add::<Page<User>>()    // registers `User_Page`
    .add::<User>()          // registers `User`
    .build()?;
```

A struct that references a generic instantiation in a field (`profile:
Page<User>`) sees its `$ref` resolved through the standard
[transitive-closure walk](nested-structs.md#explicit-transitive-closure); the builder
reports the missing target by its composed name:

```text
Err(UnresolvedReference(SchemaName("User_Page")))
```

## Primitive arguments are inlined, not referenced

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
same inline treatment applies to all eleven primitive scalars (`Int32`,
`Int64`, `UInt32`, `UInt64`, `Float`, `Double`, `Boolean`, `String`,
`Uuid`, `DateTime`, `Date`).

## Primitive scalar names are reserved

Because every reference to those eleven names is inlined, registering
your own schema under one of them would produce an entry that nothing
can ever point at. `SchemasBuilder::build` therefore rejects it:

```rust
#[derive(Schema)]
struct Int64 {              // registers as `Int64`
    value: i64,
}
```

```text
Err(ReservedSchemaName { name: SchemaName("Int64") })
```

Only an **exact** match is reserved. Composed generic names
(`Int64_Container` from `Container<i64>`) and namespaced names
(`v1.Int64`, which always carry a dot) are unaffected. There are two
remedies: rename the Rust type, or keep the name and put the type
under a `#[frieze(namespace)]` `mod`, which prefixes the registered
name:

```rust
// the attribute takes no argument: the mod's own ident is the
// namespace name
#[frieze(namespace)]
pub mod v1 {
    #[derive(Schema)]
    pub struct Int64 {      // registers as `v1.Int64`
        pub value: i64,
    }
}
```

The prefix comes from the `inventory` feature, which is on by default.
With `default-features = false` the attribute has no effect on the
registered name, so renaming is the only remedy in that configuration.

The reservation itself is feature-independent: `Uuid` is rejected even
in a build without the [`uuid1` feature](scalars.md#uuiduuid-uuid1-feature), and
`DateTime` / `Date` even without the
[`chrono04` feature](scalars.md#chrono-date-and-time-chrono04-feature), because
the inlining that makes the name unreachable happens in the boundary
conversion, which has no view of the features.

## Owned-wrapper composition

`Box<T>`, `Rc<T>`, and `Arc<T>` are
[transparent owned wrappers](owned-wrappers.md), so they do
**not** contribute to the composed name. `Box<User>`'s schema name is
`"User"`, not `"User_Box"`; `Vec<Box<Tree>>`'s element name is
`"Tree"`. This is what makes recursive type definitions
(`struct Tree { children: Vec<Box<Tree>> }`) emit a finite,
self-referencing schema instead of an unbounded `Tree_Box_Box_..."`
cascade.

## Recursive generic types

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

## Generic enums

The same rules apply to enum derive: `#[derive(Schema)]` accepts type
parameters on both the unit-variant (`type: string, enum: [...]`) and
the internally-tagged (`oneOf` with `discriminator`) branches, with a
synthesised `T: Schema` bound on every type parameter and the user's
`where` clause preserved verbatim. The composed schema name follows
the same suffix-form rule as struct derive — `Event<i64, String>` (a
two-parameter internally-tagged enum) becomes `Int64_String_Event`.

For an internally-tagged enum whose newtype variant inners are
themselves generic structs (`Container<T>`), the per-variant
`IsStructSchema` bound check accepts the concrete instantiation
because `Container<T>: IsStructSchema` holds whenever `T: Schema` does
(the struct derive carries the `IsStructSchema` impl forward through
the same `T: Schema` bound). Each generic-struct instantiation is a
distinct schema entry and must be registered explicitly alongside the
enum:

```rust
frieze::SchemasBuilder::new()
    .add::<Event<i64, String>>()    // registers `Int64_String_Event`
    .add::<Container<i64>>()        // registers `Int64_Container`
    .add::<Container<String>>()     // registers `String_Container`
    .build()?;
```

Primitive arguments are inlined the same way they are for generic
structs — `Container<i64>`'s `value: T` reference becomes
`{type: integer, format: int64}` at the leaf, without producing a
dangling `$ref: Int64` or a `components/schemas/Int64` entry.

## Rejected generic shapes

- **Lifetime parameters** (`struct Borrowed<'a> { s: &'a str }`,
  `enum Borrowed<'a> { ... }`) — rejected at macro-expansion time.
  frieze schemas describe owned data layouts, and the OAS
  representation of a borrow is undefined.
- **Const generics** (`struct ArrN<const N: usize> { ... }`,
  `enum ArrN<const N: usize> { ... }`) — rejected at macro-expansion
  time. The OAS encoding of a compile-time constant in a schema name
  or shape is not in scope.
- **Trait objects as arguments** (`Box<dyn Schema>`) — rejected by
  rustc (the `T: Schema` bound is not satisfied by `dyn Schema`).
  frieze does not synthesise a curated diagnostic for this; the
  standard rustc message is sufficient.
