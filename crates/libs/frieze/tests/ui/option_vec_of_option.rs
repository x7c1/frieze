use frieze::Schema;

#[derive(Schema)]
struct S {
    x: Option<Vec<Option<String>>>,
}

fn main() {}
