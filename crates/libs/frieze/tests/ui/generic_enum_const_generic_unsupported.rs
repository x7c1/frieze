//! Const generics on `#[derive(Schema)]` enums are rejected at
//! macro-expansion time, the same way they are for structs: OAS has no
//! representation for a compile-time constant in a schema name or
//! shape.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
enum ArrN<const N: usize> {
    Sized([i64; N]),
    Empty,
}

fn main() {}
