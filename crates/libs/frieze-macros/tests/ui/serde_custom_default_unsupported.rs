use frieze::Schema;
use serde::{Deserialize, Serialize};

fn fallback() -> i64 {
    0
}

#[derive(Schema, Serialize, Deserialize)]
struct User {
    #[serde(default = "fallback")]
    id: i64,
}

fn main() {}
