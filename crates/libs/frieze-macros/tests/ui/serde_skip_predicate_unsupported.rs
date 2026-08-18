use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct User {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

fn main() {}
