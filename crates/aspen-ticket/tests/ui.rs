#[test]
fn iroh_helpers_require_feature() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/iroh_helpers_require_feature.rs");
}

#[cfg(feature = "signed")]
#[test]
fn std_wrappers_require_feature() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/std_wrappers_require_feature.rs");
}
