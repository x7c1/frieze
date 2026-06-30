//! End-to-end: `#[frieze(namespace)]` on an inline `pub mod v1 { ... }`
//! folds the OAS key for `User` defined inside the block into the
//! `v1.User` namespace form.
//!
//! This is the central baseline for the namespace mechanism. The
//! attribute is applied to an inline mod block; the body lives in
//! source the macro sees directly. The attribute macro records the
//! namespace fact through the inventory side channel and the derived
//! `Schema::name()` body folds it in at runtime by walking
//! `module_path!()`.
//!
//! # Why this test uses an inline body rather than a file-based
//! # `pub mod v1;` declaration
//!
//! Stable Rust rejects file-based mod declarations as the *input* of
//! a proc-macro attribute (rust-lang/rust#54727 — "file modules in
//! proc macro input are unstable"). Inline mod blocks are accepted,
//! so the test exercises the inline shape. The OAS keying behaviour
//! does not depend on inline vs. file-based — `module_path!()` is the
//! same in either case and the inventory submission carries the same
//! `parent_path` / `local_name` payload — so the inline shape is a
//! faithful test of the mechanism.
//!
//! Per-binary inventory isolation: this test binary defines exactly
//! one namespace (`v1`) and exactly one `#[derive(Schema)]` type
//! (`User`), so the snapshot below is stable.
//!
//! Gated on `--features inventory`; without the feature
//! `#[frieze(namespace)]` is a transparent pass-through and the
//! coverage is the `derive_namespace_disabled` test instead.

#![cfg(feature = "inventory")]

use frieze::Schema;

#[frieze::frieze(namespace)]
pub mod v1 {
    use frieze::Schema;

    #[derive(Schema)]
    #[allow(dead_code)]
    pub struct User {
        pub id: i64,
    }
}

#[test]
fn namespace_folds_into_oas_key() {
    let s: frieze::Schemas = frieze::schemas()
        .from_inventory()
        .build()
        .expect("inventory iteration produces a closed schemas set");

    // The `Schema::name()` on `v1::User` composes
    // `v1.User` because the inventory channel records
    // `parent_path = "derive_namespace_simple"`,
    // `local_name = "v1"` for the `#[frieze(namespace)]` site, and
    // `User::name()` walks the module path
    // `"derive_namespace_simple::v1"`, keeping the `v1` segment.
    insta::assert_yaml_snapshot!(frieze::to_value(&s), @r##"
    v1.User:
      type: object
      required:
        - id
      properties:
        id:
          type: integer
          format: int64
    "##);
}

// Reference `Schema` so the `use` import is not flagged as unused
// when the test binary lacks any other call site for the trait.
const _: fn() = || {
    let _ = <v1::User as Schema>::name();
};
