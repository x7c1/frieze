# Nested structs

A field whose type is another `Schema`-deriving struct (referred to as
`U` in the [composite shapes table](composite-shapes.md)) is emitted as a `$ref` to
`#/components/schemas/<U::name()>`. The schema name is derived from the
Rust type name via the `Schema::name()` impl that `#[derive(Schema)]`
generates.

## Explicit transitive closure

Every reachable schema must be registered via `Schemas::add::<T>()`
on the same `SchemasBuilder`. The builder walks every property's type
tree and returns `Err(Error::UnresolvedReference(...))` for the first
`$ref` whose target schema is missing. Auto-discovery is intentionally
not provided — the registration list is the user's authoritative
inventory of what is exposed.

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
`Int64_Container`). See [Generic types](generics.md) below for the
composition rule and registration requirements.
