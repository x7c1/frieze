use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct S {
    #[serde(default)]
    avatar_url: Maybe<String>,
}

fn main() {}
