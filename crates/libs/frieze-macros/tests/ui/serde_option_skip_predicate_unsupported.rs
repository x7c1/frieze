use frieze::Schema;
use serde::{Deserialize, Serialize};

fn skip_name(_: &Option<String>) -> bool {
    false
}

#[derive(Schema, Serialize, Deserialize)]
struct User {
    #[serde(skip_serializing_if = "skip_name")]
    name: Option<String>,
}

fn main() {}
