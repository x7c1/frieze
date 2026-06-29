use frieze::Schema;

// Tuple variants with multiple fields are not supported: frieze
// requires a named newtype struct so the inner has a reusable OAS
// schema name.
#[derive(Schema)]
#[allow(dead_code)]
enum Bad {
    Point(i32, i32),
}

fn main() {}
