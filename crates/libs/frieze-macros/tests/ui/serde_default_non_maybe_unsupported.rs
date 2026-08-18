use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
struct User {
    #[serde(default)]
    id: i64,
}

fn main() {}
