use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Inner {
    id: i64,
}

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct Outer {
    #[serde(flatten)]
    inner: Inner,
}

fn main() {}
