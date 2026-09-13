use super::*;
use crate::input_test_support::*;

// r[verify molten.project.inherited_tracey_classification.inventory]
#[test]
fn native_definition_identity_and_path_are_preserved() {
    let root = FixtureRoot::new("classifier-native");
    root.native_specification();
    let definitions = read_definitions(root.path()).expect("native definitions");
    assert_eq!(definitions.len(), 1);
    let locations = definitions.get(REQUIREMENT_ID).expect("known definition");
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].specification, NATIVE_SPECIFICATION);
    assert_eq!(locations[0].line, 1);
    let rows = classify_baseline(&[REQUIREMENT_ID.to_string()], &definitions).expect("matching baseline");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].specification, NATIVE_SPECIFICATION);
    assert_eq!(rows[0].class, CLASS_ACCEPTED_IMPLEMENTATION_UNESTABLISHED);
    assert!(classify_baseline(&["molten.example.unknown".to_string()], &definitions).is_err());
}

// r[verify molten.project.inherited_tracey_classification.duplicate_denial]
#[test]
fn duplicate_native_definitions_still_reject() {
    let root = FixtureRoot::new("classifier-duplicate");
    root.native_specification();
    root.write(".cairn/specs/other/spec.md", &format!("r[{REQUIREMENT_ID}]\n"));
    let definitions = read_definitions(root.path()).expect("native definitions");
    let errors = classify_baseline(&[REQUIREMENT_ID.to_string()], &definitions).expect_err("duplicate definition");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("duplicate accepted definitions"));
}

#[test]
fn heading_definitions_preserve_location_and_duplicate_denial() {
    let root = FixtureRoot::new("classifier-heading");
    root.write(NATIVE_SPECIFICATION, &format!("### Requirement: Supplied lease [r[{REQUIREMENT_ID}]]\n"));
    let definitions = read_definitions(root.path()).expect("heading definition");
    let locations = definitions.get(REQUIREMENT_ID).expect("exact identifier");
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].specification, NATIVE_SPECIFICATION);
    assert_eq!(locations[0].line, 1);
    let baseline = [REQUIREMENT_ID.to_string()];
    assert_eq!(classify_baseline(&baseline, &definitions).expect("heading classification").len(), 1);
    root.write(".cairn/specs/other/spec.md", &format!("r[{REQUIREMENT_ID}]\n"));
    let duplicated = read_definitions(root.path()).expect("mixed syntax definitions");
    let errors = classify_baseline(&baseline, &duplicated).expect_err("duplicate baseline identifier");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("duplicate accepted definitions"));
}

#[test]
fn missing_or_legacy_only_definition_roots_reject() {
    let root = FixtureRoot::new("classifier-missing");
    let error = read_definitions(root.path()).expect_err("missing native root");
    assert!(error.contains("cannot read required specification directory"));
    root.write("cairn/specs/example/spec.md", &format!("r[{REQUIREMENT_ID}]\n"));
    let error = read_definitions(root.path()).expect_err("legacy-only definitions");
    assert!(error.contains("cannot read required specification directory"));
}

#[test]
fn non_directory_and_unmarked_definition_roots_reject() {
    let file_root = FixtureRoot::new("classifier-file");
    file_root.write(NATIVE_ROOT, &format!("r[{REQUIREMENT_ID}]\n"));
    let error = read_definitions(file_root.path()).expect_err("file instead of directory");
    assert!(error.contains("required specification path is not a directory"));
    let empty_root = FixtureRoot::new("classifier-empty");
    fs::create_dir_all(empty_root.path().join(NATIVE_ROOT)).expect("empty native directory");
    let error = read_definitions(empty_root.path()).expect_err("empty native tree");
    assert!(error.contains("required specification tree contains no requirement definitions"));
    empty_root.write(NATIVE_SPECIFICATION, "# No requirement definition\n");
    let error = read_definitions(empty_root.path()).expect_err("unmarked native tree");
    assert!(error.contains("required specification tree contains no requirement definitions"));
}
