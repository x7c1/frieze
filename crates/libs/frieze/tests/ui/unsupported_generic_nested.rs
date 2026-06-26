use frieze::Schema;

#[allow(dead_code)]
struct Wrapper<T>(T);

#[derive(Schema)]
#[allow(dead_code)]
struct S {
    field: Wrapper<u32>,
}

fn main() {}
