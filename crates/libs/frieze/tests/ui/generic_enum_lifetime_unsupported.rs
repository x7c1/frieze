//! Lifetime parameters on `#[derive(Schema)]` enums are rejected at
//! macro-expansion time, the same way they are for structs: frieze
//! schemas describe owned data layouts and there is no OAS
//! representation of a borrow.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
enum Borrowed<'a> {
    Holding(&'a str),
    Empty,
}

fn main() {}
