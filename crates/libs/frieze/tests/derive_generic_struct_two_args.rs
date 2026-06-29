//! Schema name composition for a two-argument generic struct:
//! `Pair<i32, f32>` produces `Int32_Float_Pair` — the type parameters
//! appear in declaration order, separated by `_`, with the base name as
//! the suffix. The primitive names follow the OAS type/format
//! convention (`f32` is `Float`; `f64` is `Double`).
//!
//! As with the single-arg primitive case, the full Schemas-build flow
//! is not exercised here (primitive references to `Int32` / `Float`
//! cannot be resolved through `Schemas` directly).

use frieze::Schema;

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
