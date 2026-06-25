//! UI tests asserting that the `#[derive(Schema)]` macro emits the expected
//! compile errors for unsupported inputs.
//!
//! Run with `TRYBUILD=overwrite cargo test --test trybuild` to regenerate
//! the expected `.stderr` files when error messages change intentionally.

#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
