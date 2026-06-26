use frieze::{Maybe, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct S {
    #[serde(skip_serializing_if = "Maybe::is_missing")]
    avatar_url: Maybe<String>,
}

fn main() {}
