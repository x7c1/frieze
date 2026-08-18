# Field shapes

`#[derive(Schema)]` recognises a fixed scalar set, optionally composed
with `Vec<T>`, `Option<T>`, and the frieze-defined `Maybe<T>` wrapper.
Field types that are themselves `Schema`-deriving structs are emitted as
`$ref` (see [Nested structs](nested-structs.md)). A `Schema`-deriving
unit-variant enum is also a valid field type; it rides on the same
`$ref` transit path (see [Unit-variant enums](unit-variant-enums.md)).

The specification is split by topic:

| Document                                                 | Topic                                                                                            |
|----------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| [scalars.md](scalars.md)                                 | The scalar set, the opt-in `uuid1` / `chrono04` scalars, and the primitive `Schema` impls        |
| [composite-shapes.md](composite-shapes.md)               | Presence × nullability: `Option` / `Vec` / `Maybe`, the `Maybe` attribute check, rejected shapes |
| [nested-structs.md](nested-structs.md)                   | `$ref` emission, automatic transitive registration, nullable references per OAS version          |
| [owned-wrappers.md](owned-wrappers.md)                   | Transparent `Box<T>` / `Rc<T>` / `Arc<T>` and recursive types                                    |
| [generics.md](generics.md)                               | Generic structs and enums, schema-name composition, reserved primitive names                     |
| [unit-variant-enums.md](unit-variant-enums.md)           | String-enum schemas from unit-variant enums                                                      |
| [internally-tagged-enums.md](internally-tagged-enums.md) | `oneOf` + `discriminator` schemas from `#[serde(tag = "...")]` enums                             |
| [wire-names.md](wire-names.md)                           | `rename` / `rename_all`, wire-name uniqueness, unsupported serde attributes                      |
| [descriptions.md](descriptions.md)                       | Doc comments to `description`                                                                    |
