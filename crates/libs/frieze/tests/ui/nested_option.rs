use frieze::Schema;

#[derive(Schema)]
struct S {
    x: Option<Option<i64>>,
}

fn main() {}
