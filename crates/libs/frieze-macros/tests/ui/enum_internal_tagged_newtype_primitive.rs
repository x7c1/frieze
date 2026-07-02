use frieze::Schema;
use serde::{Deserialize, Serialize};

// Newtype variant whose inner is a primitive scalar (`String`):
// internally-tagged variants need a `Schema`-implementing struct
// inner, not a primitive.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Text(String),
}

fn main() {}
