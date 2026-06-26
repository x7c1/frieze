use frieze::{Maybe, Schema};

#[derive(Schema)]
struct S {
    x: Maybe<Option<String>>,
}

fn main() {}
