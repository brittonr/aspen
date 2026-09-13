use super::requirement_marker;

const FIXTURE_ID: &str = "molten.example.definition";

#[test]
fn standalone_markers_keep_identity_and_version_behavior() {
    for text in [
        format!("r[{FIXTURE_ID}]"),
        format!("  r[{FIXTURE_ID}] Molten MUST retain this definition."),
        format!("r[{FIXTURE_ID}+draft]"),
    ] {
        assert_eq!(requirement_marker(&text).as_deref(), Some(FIXTURE_ID));
    }
}

#[test]
fn requirement_heading_wrappers_preserve_the_exact_identifier() {
    for text in [
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}]]"),
        format!("  ### Requirement: Supplied values [r[{FIXTURE_ID}+draft]]  "),
    ] {
        assert_eq!(requirement_marker(&text).as_deref(), Some(FIXTURE_ID));
    }
}

#[test]
fn prose_references_and_other_headings_are_not_definitions() {
    for text in [
        format!("A mention of r[{FIXTURE_ID}] is not a definition."),
        format!("### Scenario: Supplied values [r[{FIXTURE_ID}]]"),
        format!("### Requirement: `r[{FIXTURE_ID}]` in an example"),
        format!("// r[{FIXTURE_ID}]"),
        format!("r[{} {FIXTURE_ID}]", "impl"),
        format!("### Requirement: Supplied values [r[{} {FIXTURE_ID}]]", "verify"),
    ] {
        assert_eq!(requirement_marker(&text), None, "unexpected definition: {text}");
    }
}

#[test]
fn malformed_and_ambiguous_heading_wrappers_are_not_definitions() {
    for text in [
        "### Requirement: Supplied values [r[]]".to_string(),
        "### Requirement: Supplied values [r[bad/id]]".to_string(),
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}]"),
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}]]]"),
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}+draft]]]"),
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}+draft[nested]]"),
        format!("### Requirement: Supplied values [r[{FIXTURE_ID}]] trailing prose"),
        format!("### Requirement: [r[{FIXTURE_ID}]]"),
        format!("### Requirement: First [r[{FIXTURE_ID}]] second [r[{FIXTURE_ID}]]"),
    ] {
        assert_eq!(requirement_marker(&text), None, "unexpected definition: {text}");
    }
}
