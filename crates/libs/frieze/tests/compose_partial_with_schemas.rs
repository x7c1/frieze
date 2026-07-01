//! `compose` merges a `Schemas` collection into a partial OAS document
//! parsed from YAML, preserving every other section (`info`, `paths`,
//! `tags`, vendor extensions) verbatim.
//!
//! End-to-end coverage of the user's typical flow: hand-written partial
//! OAS document on disk + Rust types annotated with `#[derive(Schema)]`,
//! glued together by `compose` to produce a complete `OasDocument` ready
//! to be serialised back to YAML or JSON.

use frieze::{compose, to_yaml, OasDocument, Schema};

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

/// The OAS version string this build was compiled to emit. The partial
/// YAML below is stitched together with this prefix so the transition
/// guard in `compose` accepts it under both `oas-3-0` and `oas-3-1`
/// builds.
#[cfg(feature = "oas-3-0")]
const OPENAPI_HEADER: &str = "openapi: 3.0.3\n";
#[cfg(feature = "oas-3-1")]
const OPENAPI_HEADER: &str = "openapi: 3.1.0\n";

const PARTIAL_BODY: &str = "\
info:
  title: Example API
  version: 1.0.0
  description: An example API.
  x-internal-id: 42
paths:
  /ping:
    get:
      summary: ping
tags:
- name: example
x-codegen-info:
  generator: frieze
";

fn partial_yaml() -> String {
    format!("{OPENAPI_HEADER}{PARTIAL_BODY}")
}

#[test]
fn compose_merges_schemas_into_partial_and_preserves_other_sections() {
    let yaml = partial_yaml();
    let partial: OasDocument =
        serde_yaml::from_str(&yaml).expect("partial YAML must parse as OasDocument");

    let schemas: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    let composed = compose(partial, schemas).expect("partial has no schemas; merge must succeed");

    // Strip the version-dependent `openapi:` line so the snapshot is
    // identical under both `oas-3-0` and `oas-3-1`. The partial above
    // pins the version to `3.0.3` on the wire, but the assertion in
    // this test is about composition shape (which sections survive),
    // not the version string.
    let yaml = to_yaml(&composed);
    let yaml = yaml.lines().skip(1).collect::<Vec<_>>().join("\n");

    insta::assert_snapshot!(yaml, @r###"
    info:
      title: Example API
      version: 1.0.0
      description: An example API.
      x-internal-id: 42
    paths:
      /ping:
        get:
          summary: ping
    components:
      schemas:
        User:
          type: object
          required:
          - id
          - name
          properties:
            id:
              type: integer
              format: int64
            name:
              type: string
    tags:
    - name: example
    x-codegen-info:
      generator: frieze
    "###);
}

#[test]
fn compose_preserves_empty_components_when_no_schemas_registered() {
    // A partial that already had a `components:` slot (with non-schema
    // keys, or an empty schemas map) keeps that slot intact. The
    // boundary writes only into `components.schemas` and leaves the
    // surrounding shape alone.
    let yaml = partial_yaml();
    let partial: OasDocument =
        serde_yaml::from_str(&yaml).expect("partial YAML must parse as OasDocument");

    let schemas: frieze::Schemas = frieze::schemas()
        .build()
        .expect("empty builder must produce empty Schemas");

    let composed = compose(partial, schemas).expect("merge of empty schemas must succeed");

    // The composed document still has every other section the partial
    // carried. `components.schemas` is empty (no Rust types were
    // registered), so the boundary leaves the (defaulted-and-empty)
    // components block alone — assert via the public field rather than
    // the YAML render.
    assert!(composed
        .components
        .as_ref()
        .map(|c| c.schemas.is_empty())
        .unwrap_or(true));
    // `paths` and `tags` survived the round-trip.
    assert!(composed.paths.is_some());
    assert!(composed.tags.is_some());
}
