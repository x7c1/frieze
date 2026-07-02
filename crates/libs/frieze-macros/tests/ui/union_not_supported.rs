use frieze::Schema;

#[derive(Schema)]
union U {
    a: i64,
    b: i64,
}

fn main() {}
