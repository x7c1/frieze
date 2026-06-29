use frieze::Schema;
use serde::{Deserialize, Serialize};

// A unit-only enum should not carry `#[serde(tag = "...")]`: drop
// the attribute to emit a string-enum schema. The tagged unit-only
// form would serialise to anonymous `{<tag>: "..."}` wrappers and
// diverge from the cleaner string-enum path.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

fn main() {}
