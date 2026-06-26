use frieze::Schema;

mod inner {
    #[allow(dead_code)]
    pub struct User;
}

#[derive(Schema)]
#[allow(dead_code)]
struct S {
    user: inner::User,
}

fn main() {}
