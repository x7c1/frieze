# Architecture

frieze is a Cargo workspace. This document is the canonical
description of the crate layout, the dependency direction between the
crates, and the invariants that keep those boundaries meaningful.

## Workspace layout

```
crates/
  apps/
    frieze-cli           # bin `cargo-frieze`: the `cargo frieze generate` subcommand
  gateway/
    frieze-fs            # Filesystem gateway: package metadata, partials, outputs
    frieze-cargo         # Cargo gateway: schema collection via a scratch crate
  domain/
    frieze-model         # Domain types whose invariants are enforced by the type system
    frieze-usecase       # Boundary conversion, document composition, gateway traits, GenerateOas interactor
  libs/
    frieze-openapi       # Plain representation of the OpenAPI Specification (+ to_yaml)
    frieze-macros        # proc-macro crate
    frieze-wire          # Composition root: injects the concrete gateways into the interactors
    frieze               # User-facing API: Schema / Register traits + SchemasBuilder registry
```

## Dependency direction

```
frieze-cli     -> frieze-wire, frieze-usecase, frieze-model
frieze-wire    -> frieze-fs, frieze-cargo, frieze-usecase
frieze-fs      -> frieze-usecase, frieze-model, frieze-openapi
frieze-cargo   -> frieze-usecase, frieze-model, frieze-openapi
frieze-usecase -> frieze-model, frieze-openapi
frieze         -> frieze-model, frieze-macros
```

(`frieze-macros` has no runtime dependency on the other crates: the
tokens it emits resolve through `::frieze::__private`. `frieze` also
dev-depends on `frieze-openapi` / `frieze-usecase` for its integration
tests.)

## Invariants

1. `frieze-model` depends on nothing else within frieze (and minimally on external crates).
2. `frieze-openapi` does not know about `frieze-model` or `frieze-usecase`.
3. Only `frieze-usecase` performs the boundary conversion between `frieze-openapi` and `frieze-model`.
4. `frieze-model` validates values at constructor boundaries. Identifier-like newtypes keep private fields, while aggregate schema types expose their fields for boundary conversion; callers that mutate or construct those aggregates directly are responsible for preserving their documented invariants.
5. `frieze-macros` only touches the `Schema` / `Register` traits and the `__private` helpers defined in `frieze`; it never constructs `frieze-openapi` types, and reaches `frieze-model` constructors only through `::frieze::__private`.
6. Gateway crates (`frieze-fs`, `frieze-cargo`) implement the gateway traits defined in `frieze-usecase`; they do not know about each other.
7. `frieze-usecase` does not depend on any gateway crate — it holds only the trait definitions and the interactors written against them.
8. Concrete gateway types are known only to `frieze-wire` and to the gateway crates themselves; `frieze-cli` obtains the assembled interactor through `frieze-wire` and never names a gateway type.

## Terminology

The term **"DTO"** is **not** used here. `frieze-openapi` types are a
plain representation of the OAS specification; `frieze-model` types are
validated domain types that uphold internal invariants. Lumping them as
"DTOs" hides the responsibility difference the architecture is built
upon — refer to them by their crate-specific roles instead.
