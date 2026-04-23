#[test]
fn test_std_wrappers_require_feature() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/std_wrappers_require_feature.rs");
}
