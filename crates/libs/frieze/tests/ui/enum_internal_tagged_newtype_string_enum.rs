use frieze::Schema;
use serde::{Deserialize, Serialize};

// E-2c: newtype inner is itself a Schema-deriving enum (string enum).
// The macro emits an `IsStructSchema` trait-bound check per variant;
// since string-enum derives do not implement `IsStructSchema`, the
// bound check fails to compile.
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
