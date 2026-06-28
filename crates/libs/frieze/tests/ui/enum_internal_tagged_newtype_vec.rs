use frieze::Schema;
use serde::{Deserialize, Serialize};

// E-2b: newtype inner is a `Vec<T>` wrapper, not a struct.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Counts(Vec<i64>),
}

fn main() {}
