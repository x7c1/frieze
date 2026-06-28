use frieze::Schema;

#[derive(Schema)]
#[allow(dead_code)]
struct LoginData {
    user_id: i64,
}

#[derive(Schema)]
#[allow(dead_code)]
struct LogoutData {
    reason: String,
}

// E-1: data-carrying variants but no `#[serde(tag = "...")]`.
#[derive(Schema)]
#[allow(dead_code)]
enum Event {
    Login(LoginData),
    Logout(LogoutData),
}

fn main() {}
