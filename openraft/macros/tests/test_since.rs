#[test]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/since/fail/*.rs");
}

#[test]
#[ignore = "requires cargo-expand which is not in dev environment"]
fn pass() {
    macrotest::expand("tests/since/expand/*.rs");
}
