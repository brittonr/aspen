//! Unit-test launcher: the test binary re-executes only its peer entry test.

impl super::Launcher {
    pub(in crate::capability_qualification) fn test_harness() -> Self {
        Self::TestHarness {
            program: std::env::current_exe().expect("current unit-test executable"),
        }
    }
}
