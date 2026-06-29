use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct LoginData {
    user_id: i64,
}

// Adjacent tagging (`tag` + `content`) is not supported: the wire
// shape is a two-level wrapper that does not fit the OAS
// `discriminator` semantics. Use an internal tag without `content`
// instead.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
#[allow(dead_code)]
enum Bad {
    Login(LoginData),
}

fn main() {}
