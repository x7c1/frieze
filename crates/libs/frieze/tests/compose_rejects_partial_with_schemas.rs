//! `compose` rejects a partial whose `components.schemas` map already
//! has entries — the schemas slot must be empty so that Rust types
//! collected via `#[derive(Schema)]` are the single source of truth.
//!
//! The error carries the count of already-present schemas to help
//! authors locate the offending entries; the test pins both the
//! variant and the count.

use frieze::{compose, Error, OasDocument, Schema};

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
}

/// The OAS version string this build was compiled to emit. Used to
/// stitch a version-matching `openapi:` header onto the partial YAML
/// below, so the transition guard in `compose` accepts the document
/// under both `oas-3-0` and `oas-3-1` builds — this test's contract
/// is about the schemas-count check, not about version dispatch.
#[cfg(feature = "oas-3-0")]
const OPENAPI_HEADER: &str = "openapi: 3.0.3\n";
#[cfg(feature = "oas-3-1")]
const OPENAPI_HEADER: &str = "openapi: 3.1.0\n";

/// A partial whose `components.schemas` map carries two hand-written
/// entries. The two entries are intentionally not registered as Rust
/// types: the failure must surface from the existence of the entries
/// themselves, not from any collision check between them and the
/// builder-collected schemas.
const PARTIAL_BODY: &str = "\
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
    let yaml = format!("{OPENAPI_HEADER}{PARTIAL_BODY}");
    let partial: OasDocument =
        serde_yaml::from_str(&yaml).expect("partial YAML must parse as OasDocument");

    let schemas: frieze::Schemas = frieze::schemas()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    let err = compose(partial, schemas).expect_err("compose must reject pre-populated schemas");

    assert_eq!(err, Error::PartialAlreadyHasSchemas { count: 2 });
}
