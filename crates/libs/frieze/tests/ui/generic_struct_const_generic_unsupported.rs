//! Const generics on `#[derive(Schema)]` structs are rejected at
//! macro-expansion time: OAS has no representation for a compile-time
//! constant in a schema name or shape.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct ArrN<const N: usize> {
    items: [i64; N],
}

fn main() {}
