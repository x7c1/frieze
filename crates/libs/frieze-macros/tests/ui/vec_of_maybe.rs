use frieze::Schema;
use frieze_model::Maybe;

#[derive(Schema)]
struct S {
    x: Vec<Maybe<String>>,
}

fn main() {}
