use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    #[serde(rename(serialize = "userId", deserialize = "user_id_in"))]
    user_id: i64,
}

fn main() {}
