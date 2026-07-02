// `rename_all = "lowercase"` collapses `User` and `USER` to the same
// wire name `user`. The check catches the collision at macro-expansion
// time.

use frieze::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum Tag {
    User,
    USER,
}

fn main() {}
