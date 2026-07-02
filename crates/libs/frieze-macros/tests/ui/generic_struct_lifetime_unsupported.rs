//! Lifetime parameters on `#[derive(Schema)]` structs are rejected at
//! macro-expansion time: frieze schemas describe owned data layouts and
//! there is no OAS representation of a borrow.

use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct Borrowed<'a> {
    s: &'a str,
}

fn main() {}
