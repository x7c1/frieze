use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
enum Status {
    Active,
    #[serde(rename = "Active")]
    Inactive,
}

fn main() {}
