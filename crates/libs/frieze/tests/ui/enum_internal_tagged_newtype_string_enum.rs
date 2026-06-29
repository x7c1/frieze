use frieze::Schema;
use serde::{Deserialize, Serialize};

// Newtype variant whose inner is itself an enum-derived `Schema`
// (here a string-enum): internally-tagged variants need a struct
// inner so the synthesized tag field can be merged into the inner
// object. The macro emits an `IsStructSchema` trait-bound check
// per variant; since enum derives do not implement
// `IsStructSchema`, the bound check fails to compile.
#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
enum Status {
    Active,
    Inactive,
}

#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[allow(dead_code)]
enum Payload {
    Mode(Status),
}

fn main() {}
