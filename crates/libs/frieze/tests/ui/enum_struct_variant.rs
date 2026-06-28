use frieze::Schema;

// E-3: struct variants are never supported (any mode, with or without
// `#[serde(tag = "...")]`).
#[derive(Schema)]
#[allow(dead_code)]
enum Event {
    Login { user_id: i64 },
    Logout,
}

fn main() {}
