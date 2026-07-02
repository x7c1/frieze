use frieze::Schema;
use frieze_model::Maybe;

#[derive(Schema)]
struct S {
    x: Option<Maybe<String>>,
}

fn main() {}
