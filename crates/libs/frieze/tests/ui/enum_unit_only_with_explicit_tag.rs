use frieze::Schema;
use serde::{Deserialize, Serialize};

// E-7: a unit-only enum should not carry `#[serde(tag = "...")]`.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

fn main() {}
