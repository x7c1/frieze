# Composite shapes (presence x nullability)

OpenAPI optionality has two **independent** axes: **presence** controls
whether the field name appears in the schema's `required` array, and
**nullability** controls whether the value may be `null`. The
combinations map to the following Rust shapes. `T` stands for any
scalar from [scalars.md](scalars.md); `U` stands for another
`Schema`-deriving struct:

| Rust shape                                                            | Presence | Nullability                       |
|-----------------------------------------------------------------------|----------|-----------------------------------|
| `T`                                                                   | required | non-nullable                      |
| `Option<T>` (serde default)                                           | required | nullable                          |
| `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable                      |
| `Maybe<T>`                                                            | optional | nullable                          |
| `Vec<T>`                                                              | required | array, items as `T`               |
| `Vec<Option<T>>`                                                      | required | array, nullable items             |
| `Option<Vec<T>>`                                                      | required | nullable array                    |
| `Option<Vec<T>>` + `#[serde(skip_serializing_if = "Option::is_none")]` | optional | non-nullable array                |
| `Option<Vec<Option<T>>>`                                              | required | nullable array, nullable items    |
| `Option<Vec<Option<T>>>` + `#[serde(skip_serializing_if = "Option::is_none")]` | optional | non-nullable array, nullable items |
| `Maybe<Vec<T>>`                                                       | optional | nullable array                    |
| `Maybe<Vec<Option<T>>>`                                               | optional | nullable array, nullable items    |
| `U` (another `Schema`-deriving struct)                                | required | `$ref` to `U`                     |
| `Option<U>` (serde default)                                           | required | nullable `$ref`                   |
| `Option<U>` + `#[serde(skip_serializing_if = "Option::is_none")]`     | optional | non-nullable `$ref`               |
| `Maybe<U>`                                                            | optional | nullable `$ref`                   |
| `Vec<U>`                                                              | required | array of `$ref`                   |
| `Vec<Option<U>>`                                                      | required | array of nullable `$ref`          |
| `Option<Vec<U>>`                                                      | required | nullable array of `$ref`          |
| `Option<Vec<U>>` + `#[serde(skip_serializing_if = "Option::is_none")]` | optional | non-nullable array of `$ref`      |
| `Option<Vec<Option<U>>>`                                              | required | nullable array of nullable `$ref` |
| `Option<Vec<Option<U>>>` + `#[serde(skip_serializing_if = "Option::is_none")]` | optional | non-nullable array of nullable `$ref` |
| `Maybe<Vec<U>>`                                                       | optional | nullable array of `$ref`          |
| `Maybe<Vec<Option<U>>>`                                               | optional | nullable array of nullable `$ref` |

## Notes

- **`Option<T>` is required-and-nullable by default**, because serde
  emits `None` as `null` and expects the key to be present. This is
  surprising if you read `Option` as "may be omitted" — to get
  **optional + non-nullable**, pair `Option<T>` with the standard
  `#[serde(skip_serializing_if = "Option::is_none")]` attribute. The
  derive inspects that attribute and switches branches accordingly.
- The same `Option::is_none` rule applies when `T` is an array:
  `Option<Vec<T>>` is a required nullable array by default and an optional
  non-nullable array with the attribute.
- **`Maybe<T>` is the dedicated three-state type** for "missing / null /
  present" — the one combination not expressible by `Option<T>` alone.
  Defined in `frieze-model` (`use frieze_model::Maybe;`). Add
  `#[serde(default, skip_serializing_if = "Maybe::is_missing")]` on the
  field to make missing-key handling work in both directions.
- **Nullability lives on the type tree** (`PropertyType::Nullable`),
  not on the property as a whole. That is how `Vec<Option<T>>` becomes
  an array of nullable items rather than a nullable array.

## Compile-time validation of `Maybe<T>` fields

`Maybe<T>` (including `Maybe<Vec<T>>`) only behaves correctly under serde when paired with the
attribute `#[serde(default, skip_serializing_if = "Maybe::is_missing")]`.
The `#[derive(Schema)]` macro enforces this: a `Maybe<T>` field without
both `default` **and** `skip_serializing_if = "Maybe::is_missing"` is a
compile error pointing at the offending field. This prevents schemas
from being silently inconsistent with their serialised form.

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
