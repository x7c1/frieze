use frieze::Schema;
use frieze_model::Maybe;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct S {
    avatar_url: Maybe<String>,
}

fn main() {}
