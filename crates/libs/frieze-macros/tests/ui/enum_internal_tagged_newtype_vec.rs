use frieze::Schema;
use serde::{Deserialize, Serialize};

// Newtype variant whose inner is a `Vec<T>` wrapper, not a struct:
// internally-tagged variants need a `Schema`-implementing struct
// inner, not a known wrapper.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Counts(Vec<i64>),
}

fn main() {}
