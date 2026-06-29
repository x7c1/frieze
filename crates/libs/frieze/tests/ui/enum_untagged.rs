use frieze::Schema;
use serde::{Deserialize, Serialize};

// `#[serde(untagged)]` enums are not supported: a single OAS
// schema cannot dispatch variants without a discriminator. Use an
// internal tag instead.
#[derive(Schema, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Bad {
    A,
    B,
}

fn main() {}
