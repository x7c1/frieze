use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct BarData {
    value: i64,
}

// E-2a: unit variant mixed with a data-carrying variant under
// `#[serde(tag = "...")]`.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Foo,
    Bar(BarData),
}

fn main() {}
