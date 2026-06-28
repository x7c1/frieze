use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

fn main() {}
