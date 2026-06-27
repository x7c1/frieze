use frieze::Schema;

#[derive(Schema)]
enum Event {
    Login(i64),
    Logout,
}

fn main() {}
