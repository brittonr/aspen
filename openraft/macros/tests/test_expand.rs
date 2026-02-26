#[test]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/expand/fail/*.rs");
}

#[test]
#[ignore = "requires cargo-expand which is not in dev environment"]
fn pass() {
    macrotest::expand("tests/expand/expand/*.rs");
}
