//! Bounded structural traversal, including `Embedded` payloads decoded from the wire.

use alloc::string::ToString;

fn symbol_needle(kind: crate::inspect::TokenKind, token: &str) -> bool {
    kind == crate::inspect::TokenKind::Symbol && token == "needle"
}

fn path_text(found: &crate::inspect::TokenMatch) -> alloc::vec::Vec<alloc::string::String> {
    found.path.iter().map(ToString::to_string).collect()
}

fn embedded_layers(inner: crate::model::PackedValue, layers: usize) -> crate::model::PackedValue {
    (0..layers).fold(inner, |value, _| crate::model::PackedValue::embedded(value))
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn markers_inside_decoded_embedded_values_are_detected() {
    // `[#:<secret> #:<host-path>]` straight from the wire.
    let bytes = b"\xb5\x86\xb4\xb3\x06secret\x84\x86\xb4\xb3\x09host-path\x84\x84";
    let value = super::support::decode(bytes).expect("canonical embedded values");
    let secret = crate::inspect::catalog::find_sensitive(&value).expect("traverse").expect("secret");
    assert_eq!(secret.kind, crate::inspect::TokenKind::RecordLabel);
    assert_eq!(secret.token, "secret");
    assert_eq!(path_text(&secret), ["$", "item[0]", "embedded"]);
    let ambient = crate::inspect::catalog::find_ambient_job_token(&value).expect("traverse").expect("host path");
    assert_eq!(path_text(&ambient), ["$", "item[1]", "embedded"]);

    let inert = crate::model::PackedValue::embedded(crate::model::PackedValue::string("<secret>"));
    assert_eq!(crate::inspect::catalog::find_sensitive(&inert), Ok(None));
}

#[test]
fn content_refs_are_found_through_embedded_layers() {
    let target = super::support::fixture_ref("traversal-target");
    let value = embedded_layers(crate::model::PackedValue::string(target.as_str()), 3);
    let found = crate::inspect::catalog::find_content_ref(&value, &target).expect("traverse").expect("ref");
    assert_eq!(found.kind, crate::inspect::TokenKind::ContentRef);
    assert_eq!(found.path.len(), 4);
    let other = super::support::fixture_ref("other");
    assert_eq!(crate::inspect::catalog::find_content_ref(&value, &other), Ok(None));
}

#[test]
fn preorder_paths_cover_records_sequences_and_dictionaries() {
    let dictionary: crate::model::collections::PackedDictionary =
        [(crate::model::PackedValue::symbol("k"), crate::model::PackedValue::symbol("needle"))]
            .into_iter()
            .collect();
    let value = crate::model::PackedValue::symbol_record("outer", alloc::vec![crate::model::PackedValue::Dictionary(
        dictionary
    )]);
    let all = crate::inspect::TokenScope::ALL;
    let found = crate::inspect::find_match(&value, all, crate::inspect::TraversalLimits::DEFAULT, symbol_needle)
        .expect("traverse")
        .expect("needle");
    assert_eq!(path_text(&found), ["$", "field[0]", "entry[0].value"]);
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn budgets_admit_the_exact_bound_and_deny_one_past() {
    let value = embedded_layers(crate::model::PackedValue::symbol("other"), 3);
    let all = crate::inspect::TokenScope::ALL;
    let exact = crate::inspect::TraversalLimits {
        max_nodes: 4,
        max_depth: 4,
    };
    assert_eq!(crate::inspect::find_match(&value, all, exact, symbol_needle), Ok(None));
    let shallow = crate::inspect::TraversalLimits { max_depth: 3, ..exact };
    assert_eq!(
        crate::inspect::find_match(&value, all, shallow, symbol_needle),
        Err(crate::inspect::TraversalError::DepthExceeded { limit: 3 })
    );
    let small = crate::inspect::TraversalLimits { max_nodes: 3, ..exact };
    assert_eq!(
        crate::inspect::find_match(&value, all, small, symbol_needle),
        Err(crate::inspect::TraversalError::NodesExceeded { limit: 3 })
    );
    assert_eq!(
        crate::inspect::TraversalError::DepthExceeded { limit: 3 }.to_string(),
        "structural Preserves scan exceeded depth 3"
    );
}

#[test]
fn byte_strings_are_offered_as_their_content_refs() {
    let value = crate::model::PackedValue::ByteString(alloc::vec![1, 2, 3]);
    let expected = crate::identity::ContentRef::of_bytes(&[1, 2, 3]);
    let all = crate::inspect::TokenScope::ALL;
    let found = crate::inspect::find_match(&value, all, crate::inspect::TraversalLimits::DEFAULT, |kind, token| {
        kind == crate::inspect::TokenKind::ByteString && token == expected.as_str()
    });
    assert!(matches!(found, Ok(Some(_))));
}
