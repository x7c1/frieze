use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct BarData {
    value: i64,
}

// Unit variant mixed with a data-carrying variant under
// `#[serde(tag = "...")]`: the wire shape of `Foo` would be
// `{"kind": "Foo"}`, indistinguishable from an empty struct variant.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Foo,
    Bar(BarData),
}

fn main() {}
