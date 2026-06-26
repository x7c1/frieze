use frieze::{Maybe, Schema};

#[derive(Schema)]
struct S {
    x: Option<Maybe<String>>,
}

fn main() {}
