//! `rename_all = "snake_case"` is a no-op for struct field names that
//! are already snake_case (serde's field rule doesn't synthesise word
//! boundaries from camelCase / PascalCase input — only the variant
//! rule does). This test pins the no-op behaviour so the field/variant
//! split in `RenameAll::apply` cannot silently regress and start
//! rewriting snake_case field idents.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
struct Account {
    account_id: i64,
    display_name: String,
}

#[test]
fn struct_rename_all_snake_case_is_noop_for_snake_case_idents() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Account>()
        .build()
        .expect("schemas build should succeed for valid input");

    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r###"
    Account:
      type: object
      required:
        - account_id
        - display_name
      properties:
        account_id:
          type: integer
          format: int64
        display_name:
          type: string
    "###);
}
