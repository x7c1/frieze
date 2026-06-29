use frieze::Schema;

// Struct variants (named fields) are never supported, with or
// without `#[serde(tag = "...")]`: their anonymous shape has no
// reusable OAS schema name, so frieze requires a named newtype.
#[derive(Schema)]
#[allow(dead_code)]
enum Event {
    Login { user_id: i64 },
    Logout,
}

fn main() {}
