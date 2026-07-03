// Primitive scalars (`i32`, `i64`, ... `String`) implement `Schema` so
// they can appear as generic arguments (`Box<i64>`, `Page<String>`),
// but they intentionally do **not** implement `IsRegistrable`. Calling
// `Schemas::add::<i64>()` therefore fails to compile with the curated
// `#[diagnostic::on_unimplemented]` message attached to `IsRegistrable`.

fn main() {
    let _ = frieze::SchemasBuilder::new().add::<i64>();
}
