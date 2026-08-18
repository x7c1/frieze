# Nested structs

A field whose type is another `Schema`-deriving struct (referred to as
`U` in the [composite shapes table](composite-shapes.md)) is emitted as a `$ref` to
`#/components/schemas/<U::name()>`. The schema name is derived from the
Rust type name via the `Schema::name()` impl that `#[derive(Schema)]`
generates.

## Automatic transitive registration

Register one root with `SchemasBuilder::add::<T>()`; the `Register`
implementation emitted by `#[derive(Schema)]` recursively registers every
struct, enum, and concrete generic instantiation reachable through its
fields. Repeated and recursive paths are idempotent, so a self-reference
terminates at the first already-registered schema name.

With the default `inventory` feature, every non-generic derived type is
also submitted as a root and `SchemasBuilder::from_inventory()` walks the
same transitive registration path. Generic types cannot be inventory roots,
but a concrete instantiation reached from a non-generic root is still
registered automatically. An explicit `add` is only needed for a root that
inventory cannot supply, such as an otherwise-unreachable generic
instantiation or a type with hand-written `Schema` / `Register` impls.

After registration, `build()` verifies every `$ref` target and returns
`Error::UnresolvedReference` if a hand-written registration path omitted a
dependency. Derived registration walks its field types automatically, so
ordinary derived nesting does not require one `add` call per type.

## Nullable references per OAS version

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

## Restrictions on field-position types

The macro rejects the following user-written forms as compile errors:

- **Qualified paths** (`mymod::User`) — bring the type into scope with
  a `use` statement first.

Generic arguments on the user type (`Foo<u32>`, `Page<User>`,
`Container<i64>`) are accepted; the field's `$ref` target is the
**composed schema name** of the instantiation (`Int32_Foo`, `User_Page`,
`Int64_Container`). See [Generic types](generics.md) for the composition
rule and root registration cases.
