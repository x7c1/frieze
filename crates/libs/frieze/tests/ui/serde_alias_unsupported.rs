use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[allow(dead_code)]
struct User {
    #[serde(alias = "user_id")]
    id: i64,
}

fn main() {}
