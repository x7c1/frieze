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

// Data-carrying variants without `#[serde(tag = "...")]`: the macro
// requires an internal tag so the wire shape carries a discriminator.
#[derive(Schema)]
#[allow(dead_code)]
enum Event {
    Login(LoginData),
    Logout(LogoutData),
}

fn main() {}
