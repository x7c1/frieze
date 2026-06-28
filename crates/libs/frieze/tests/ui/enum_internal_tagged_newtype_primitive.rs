use frieze::Schema;
use serde::{Deserialize, Serialize};

// E-2b: newtype inner is a primitive scalar (`String`).
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Text(String),
}

fn main() {}
