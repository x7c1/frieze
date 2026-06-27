use frieze::Schema;

#[derive(Schema)]
enum Event {
    Login { user_id: i64 },
    Logout,
}

fn main() {}
