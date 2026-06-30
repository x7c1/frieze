//! A multi-line `///` block joins its lines with `\n`, and the YAML
//! emitter must render any such string as a literal block scalar (`|-`)
//! rather than a quoted-and-escaped scalar. This is the chosen form for
//! every multi-line string the schema YAML emits: the
//! quoted form (`"a\nb"`) is wire-equivalent but unreadable for the
//! CommonMark prose, bullet lists, and multi-line examples that
//! `description` carries in practice.
//!
//! The snapshot below pins that contract end-to-end through the
//! user-facing `frieze::to_yaml` boundary, so regressions in either the
//! Value tree or the YAML backend are caught.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Note {
    /// The first line of the body.
    /// The second line of the body.
    body: String,
}

#[test]
fn multi_line_doc_comment_emits_as_literal_block_scalar() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Note>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Note:
          type: object
          required:
          - body
          properties:
            body:
              type: string
              description: |-
                The first line of the body.
                The second line of the body.
    ");
}
