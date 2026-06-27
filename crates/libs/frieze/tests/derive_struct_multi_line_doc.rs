//! A multi-line `///` block joins its lines with `\n` — the resulting
//! `description` carries both source lines verbatim. The YAML emitter
//! is free to pick between a quoted scalar (`"a\nb"`) and a block
//! scalar (`|-`); either is wire-equivalent. This snapshot pins the
//! current chosen form to catch regressions in newline handling.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Note {
    /// The first line of the body.
    /// The second line of the body.
    body: String,
}

#[test]
fn multi_line_doc_comment_carries_newlines_through() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Note>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r#"
    Note:
      type: object
      required:
        - body
      properties:
        body:
          type: string
          description: "The first line of the body.\nThe second line of the body."
    "#);
}
