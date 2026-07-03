//! `compose` rejects a partial whose `components.schemas` map already
//! has entries — the schemas slot must be empty so that Rust types
//! collected via `#[derive(Schema)]` are the single source of truth.
//!
//! The error carries the count of already-present schemas to help
//! authors locate the offending entries; the test pins both the
//! variant and the count.

use frieze::Schema;
use frieze_model::Error;
use frieze_openapi::Document;
use frieze_usecase::compose;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

/// A partial whose `components.schemas` map carries two hand-written
/// entries. The two entries are intentionally not registered as Rust
/// types: the failure must surface from the existence of the entries
/// themselves, not from any collision check between them and the
/// builder-collected schemas. The `openapi:` header is a fixed
/// `3.0.3` — this test's contract is the schemas-count check, not
/// version dispatch.
const PARTIAL_YAML: &str = "\
openapi: 3.0.3
info:
  title: Has schemas
  version: 0.1.0
components:
  schemas:
    Pre:
      type: string
    OtherPre:
      type: integer
      format: int64
";

#[test]
fn compose_rejects_partial_that_already_carries_schemas() {
    let partial: Document =
        serde_yaml::from_str(PARTIAL_YAML).expect("partial YAML must parse as Document");

    let schemas: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    let err = compose(partial, schemas).expect_err("compose must reject pre-populated schemas");

    assert_eq!(err, Error::PartialAlreadyHasSchemas { count: 2 });
}
