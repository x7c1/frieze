//! `uuid::Uuid` field shapes under OAS 3.1, behind the opt-in `uuid1`
//! feature. `Option<Uuid>` inlines to a nullable scalar, which 3.1
//! encodes by folding `"null"` into the `type` sequence.
#![cfg(feature = "uuid1")]

use frieze::Schema;
use uuid::Uuid;

mod common;

#[derive(Schema)]
#[allow(dead_code)] // Fields are read by the derive at compile time, not at runtime.
struct Session {
    id: Uuid,
    previous_id: Option<Uuid>,
    related_ids: Vec<Uuid>,
}

#[test]
fn uuid_fields_render_as_string_with_uuid_format_under_oas_3_1() {
    let s: frieze_model::Schemas = frieze::SchemasBuilder::new()
        .add::<Session>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml_3_1(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Session:
          type: object
          required:
          - id
          - previous_id
          - related_ids
          properties:
            id:
              type: string
              format: uuid
            previous_id:
              type:
              - string
              - 'null'
              format: uuid
            related_ids:
              type: array
              items:
                type: string
                format: uuid
    ");
}
