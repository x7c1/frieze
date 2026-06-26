use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct S {
    avatar_url: Maybe<String>,
}

fn main() {}
