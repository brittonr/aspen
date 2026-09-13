use super::*;
use crate::input_test_support::*;

// r[verify molten.project.inherited_tracey_debt.fixtures]
#[test]
fn native_specifications_resolve_current_references() {
    let root = FixtureRoot::new("guard-native");
    root.native_specification();
    root.write("src/lib.rs", &format!("r[{} {REQUIREMENT_ID}]\n", "verify"));
    root.write("baseline.txt", "");
    assert_eq!(
        read_requirements(root.path()).expect("native requirements"),
        [REQUIREMENT_ID.to_string()].into_iter().collect()
    );
    assert_eq!(run(&arguments(&root)), Ok(()));
}

// r[verify molten.project.inherited_tracey_debt.fixtures]
#[test]
fn missing_or_legacy_only_specifications_reject() {
    let root = FixtureRoot::new("guard-missing");
    let missing = read_requirements(root.path()).expect_err("missing native root");
    assert!(missing.contains("cannot read required specification directory"));
    root.write("cairn/specs/example/spec.md", &format!("r[{REQUIREMENT_ID}]\n"));
    let legacy = read_requirements(root.path()).expect_err("legacy root is not native acceptance");
    assert!(legacy.contains("cannot read required specification directory"));
}

// r[verify molten.project.inherited_tracey_debt.fixtures]
#[test]
fn non_directory_and_unmarked_specifications_reject() {
    let file_root = FixtureRoot::new("guard-file");
    file_root.write(NATIVE_ROOT, &format!("r[{REQUIREMENT_ID}]\n"));
    let error = read_requirements(file_root.path()).expect_err("file instead of directory");
    assert!(error.contains("required specification path is not a directory"));
    let empty_root = FixtureRoot::new("guard-empty");
    fs::create_dir_all(empty_root.path().join(NATIVE_ROOT)).expect("empty native directory");
    let error = read_requirements(empty_root.path()).expect_err("empty accepted tree");
    assert!(error.contains("required specification tree contains no requirement definitions"));
    empty_root.write(NATIVE_SPECIFICATION, "# No requirement definition\n");
    let error = read_requirements(empty_root.path()).expect_err("unmarked accepted tree");
    assert!(error.contains("required specification tree contains no requirement definitions"));
}

// r[verify molten.project.inherited_tracey_debt.growth_denial]
#[test]
fn unknown_references_still_reject_after_native_input_admission() {
    let root = FixtureRoot::new("guard-unknown");
    root.native_specification();
    root.write("src/lib.rs", &format!("r[{} molten.example.unknown]\n", "verify"));
    root.write("baseline.txt", &format!("{REQUIREMENT_ID}\n"));
    assert!(read_requirements(root.path()).expect("native input").contains(REQUIREMENT_ID));
    assert_eq!(run(&arguments(&root)), Err("dangling traceability references are not permitted".to_string()));
}

fn arguments(root: &FixtureRoot) -> Vec<String> {
    vec![
        OPTION_ROOT.to_string(),
        root.path().display().to_string(),
        OPTION_BASELINE.to_string(),
        root.path().join("baseline.txt").display().to_string(),
    ]
}
