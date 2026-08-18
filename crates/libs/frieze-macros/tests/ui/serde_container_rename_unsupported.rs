use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename = "WireUser")]
struct User {
    id: i64,
}

fn main() {}
