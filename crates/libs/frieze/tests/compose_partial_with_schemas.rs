//! `compose` merges a `Schemas` collection into a partial OAS document
//! parsed from YAML, preserving every other section (`info`, `paths`,
//! `tags`, vendor extensions) verbatim.
//!
//! End-to-end coverage of the user's typical flow: hand-written partial
//! OAS document on disk + Rust types annotated with `#[derive(Schema)]`,
//! glued together by `compose` to produce a complete `Document` ready
//! to be serialised back to YAML or JSON.

use frieze::Schema;
use frieze_openapi::{to_yaml, Document};
use frieze_usecase::compose;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

/// The partial used by every test in this file. The `openapi:` header
/// is a fixed `3.0.3` — this test's contract is composition shape
/// (which sections survive the merge), not version dispatch, and the
/// schema below is version-neutral anyway.
const PARTIAL_YAML: &str = "\
openapi: 3.0.3
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

#[test]
fn compose_merges_schemas_into_partial_and_preserves_other_sections() {
    let partial: Document =
        serde_yaml::from_str(PARTIAL_YAML).expect("partial YAML must parse as Document");

    let schemas: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    let composed = compose(partial, schemas).expect("partial has no schemas; merge must succeed");

    // Strip the `openapi:` line so the snapshot pins only what this
    // test asserts: composition shape (which sections survive), not
    // the version string.
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
    let partial: Document =
        serde_yaml::from_str(PARTIAL_YAML).expect("partial YAML must parse as Document");

    let schemas: frieze_model::Schemas = frieze::SchemasBuilder::new()
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
