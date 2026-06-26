use frieze::Schema;

#[derive(Schema)]
struct S {
    x: Vec<Option<String>>,
}

fn main() {}
