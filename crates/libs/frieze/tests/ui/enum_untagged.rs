use frieze::Schema;
use serde::{Deserialize, Serialize};

// E-5: `#[serde(untagged)]` enums are not supported.
#[derive(Schema, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Bad {
    A,
    B,
}

fn main() {}
