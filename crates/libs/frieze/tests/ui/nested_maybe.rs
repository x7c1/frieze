use frieze::{Maybe, Schema};

#[derive(Schema)]
struct S {
    x: Maybe<Maybe<String>>,
}

fn main() {}
