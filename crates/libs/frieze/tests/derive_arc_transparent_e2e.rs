//! `Arc<T>` is transparent end-to-end: a field of type `Arc<User>` is
//! emitted as `$ref: User`, no synthetic entry is generated. Parallel to
//! `derive_box_transparent_e2e.rs`.

use std::sync::Arc;

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct User {
    id: i64,
    name: String,
}

#[derive(Schema)]
#[allow(dead_code)]
struct Owner {
    shared: Arc<User>,
}

#[test]
fn arc_field_renders_as_inner_ref_with_no_arc_entry() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Owner>()
        .add::<User>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    Owner:
      type: object
      required:
        - shared
      properties:
        shared:
          $ref: "#/components/schemas/User"
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
    "##);
}
