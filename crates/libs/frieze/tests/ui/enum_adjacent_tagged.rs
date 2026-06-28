use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct LoginData {
    user_id: i64,
}

// E-6: adjacent tagging (`tag` + `content`) is not supported.
#[derive(Schema, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
#[allow(dead_code)]
enum Bad {
    Login(LoginData),
}

fn main() {}
