use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(crate = "serde")]
struct User {
    id: i64,
}

fn main() {}
