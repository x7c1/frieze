//! One build, both OAS versions: the OAS version is per-document
//! runtime data (`Document.oas_version`), not a compile-time choice,
//! so the same schema set can be emitted as an OAS 3.0 document and an
//! OAS 3.1 document side by side.
//!
//! The struct under test carries a nullable field (`Option<String>`,
//! serde's required-and-nullable default) so the two outputs actually
//! diverge: 3.0 encodes the nullability as `nullable: true`, 3.1 as
//! the 2-element `type` sequence.

use std::collections::BTreeMap;

use frieze::Schema;
use frieze_openapi::{to_yaml, Info, Version};
use frieze_usecase::from_schemas;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    bio: Option<String>,
}

fn build_schemas() -> frieze_model::Schemas {
    frieze::SchemasBuilder::new()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input")
}

fn info() -> Info {
    Info {
        title: "snapshot test".to_string(),
        version: "0.0.0".to_string(),
        description: None,
        extensions: BTreeMap::new(),
    }
}

#[test]
fn the_same_build_serializes_a_3_0_and_a_3_1_document_side_by_side() {
    let document_3_0 = from_schemas(info(), build_schemas(), Version::V3_0);
    let document_3_1 = from_schemas(info(), build_schemas(), Version::V3_1);

    // Serialize the 3.1 document first, then the 3.0 one — proving the
    // emitted shape depends only on each document's own version, not
    // on any ambient (build-wide or ordering-dependent) state.
    let yaml_3_1 = to_yaml(&document_3_1);
    let yaml_3_0 = to_yaml(&document_3_0);

    insta::assert_snapshot!(yaml_3_0, @"
    openapi: 3.0.3
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        User:
          type: object
          required:
          - id
          - bio
          properties:
            id:
              type: integer
              format: int64
            bio:
              type: string
              nullable: true
    ");

    insta::assert_snapshot!(yaml_3_1, @"
    openapi: 3.1.0
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        User:
          type: object
          required:
          - id
          - bio
          properties:
            id:
              type: integer
              format: int64
            bio:
              type:
              - string
              - 'null'
    ");
}
