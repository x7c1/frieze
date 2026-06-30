//! Schema name composition and end-to-end registration for a
//! two-argument generic struct: `Pair<i32, f32>` produces
//! `Int32_Float_Pair` — the type parameters appear in declaration
//! order, separated by `_`, with the base name as the suffix. The
//! primitive names follow the OAS type/format convention (`f32` is
//! `Float`; `f64` is `Double`).
//!
//! Both primitive fields are inlined at the leaf position (no
//! `components/schemas/Int32` or `components/schemas/Float` entry is
//! emitted), so `Schemas::add::<Pair<i32, f32>>().build()` succeeds.

use frieze::Schema;

mod common;

#[derive(Schema)]
#[allow(dead_code)]
struct Pair<A, B> {
    fst: A,
    snd: B,
}

#[test]
fn pair_two_primitives_uses_suffix_form() {
    assert_eq!(<Pair<i32, f32> as Schema>::name(), "Int32_Float_Pair");
    assert_eq!(<Pair<i32, f64> as Schema>::name(), "Int32_Double_Pair");
}

#[test]
fn pair_argument_order_is_significant() {
    // `Pair<i64, String>` and `Pair<String, i64>` produce distinct
    // schema names, in argument declaration order.
    assert_eq!(<Pair<i64, String> as Schema>::name(), "Int64_String_Pair");
    assert_eq!(<Pair<String, i64> as Schema>::name(), "String_Int64_Pair");
}

#[test]
fn pair_two_primitives_inlines_both_fields() {
    let s: frieze::Schemas = frieze::schemas()
        .add::<Pair<i32, f64>>()
        .build()
        .expect("schemas build inlines both primitive references");

    insta::assert_snapshot!(common::snapshot_yaml(s), @"
    openapi: X.Y.Z
    info:
      title: snapshot test
      version: 0.0.0
    components:
      schemas:
        Int32_Double_Pair:
          type: object
          required:
          - fst
          - snd
          properties:
            fst:
              type: integer
              format: int32
            snd:
              type: number
              format: double
    ");
}
