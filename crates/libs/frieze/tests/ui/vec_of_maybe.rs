use frieze::{Maybe, Schema};

#[derive(Schema)]
struct S {
    x: Vec<Maybe<String>>,
}

fn main() {}
