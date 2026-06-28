use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    #[serde(rename(deserialize = "userId"))]
    user_id: i64,
}

fn main() {}
