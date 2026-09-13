use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub(super) const NATIVE_ROOT: &str = ".cairn/specs";
pub(super) const NATIVE_SPECIFICATION: &str = ".cairn/specs/example/spec.md";
pub(super) const REQUIREMENT_ID: &str = "molten.example.native";

pub(super) struct FixtureRoot {
    path: PathBuf,
}

impl FixtureRoot {
    pub(super) fn new(case: &str) -> Self {
        let path = std::env::temp_dir().join(format!("molten-tracey-{case}-{}", std::process::id()));
        fs::create_dir(&path).expect("create a new test-owned root without replacing existing data");
        Self { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn write(&self, relative: &str, text: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture parent directory");
        fs::write(path, text).expect("fixture bytes");
    }

    pub(super) fn native_specification(&self) {
        self.write(NATIVE_SPECIFICATION, &format!("r[{REQUIREMENT_ID}]\n"));
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("remove only the test-owned root");
    }
}
