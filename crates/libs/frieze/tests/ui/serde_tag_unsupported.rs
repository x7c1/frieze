use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

fn main() {}
