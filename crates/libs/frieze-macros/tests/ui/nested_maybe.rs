use frieze::Schema;
use frieze_model::Maybe;

#[derive(Schema)]
struct S {
    x: Maybe<Maybe<String>>,
}

fn main() {}
