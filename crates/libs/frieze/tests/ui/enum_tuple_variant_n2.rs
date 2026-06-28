use frieze::Schema;

// E-4: tuple variants with multiple fields are not supported.
#[derive(Schema)]
#[allow(dead_code)]
enum Bad {
    Point(i32, i32),
}

fn main() {}
