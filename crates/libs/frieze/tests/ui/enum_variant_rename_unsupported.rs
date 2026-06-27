use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
enum Status {
    Active,
    #[serde(rename = "off")]
    Inactive,
}

fn main() {}
