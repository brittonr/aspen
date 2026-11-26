//! Builders against hand-written canonical bytes, and builder/reader round trips.

#[test]
fn builders_write_the_canonical_packed_bytes() {
    // `<a #t 255 "x" []>`: record, symbol label, boolean, integer, string, empty sequence.
    let value = super::record("a", alloc::vec![
        super::bool_value(true),
        super::u64_value(255),
        super::string("x"),
        super::sequence(alloc::vec::Vec::new()),
    ]);
    let expected = [
        0xb4, 0xb3, 0x01, b'a', 0x81, 0xb0, 0x02, 0x00, 0xff, 0xb1, 0x01, b'x', 0xb5, 0x84, 0x84,
    ];
    assert_eq!(crate::writer::encode_canonical(&value), expected);
    let reference = alloc::format!("blake3:{}", blake3::hash(&expected).to_hex());
    assert_eq!(super::canonical_hash(&value), reference);
    assert_eq!(super::content_ref_from_bytes(&expected), reference);
}

#[test]
fn optional_refs_and_check_sets_have_their_record_shapes() {
    assert_eq!(super::optional_ref_value(None), super::record("none", alloc::vec::Vec::new()));
    assert_eq!(super::optional_ref_value(Some("r")), super::record("some", alloc::vec![super::string("r")]));
    let checks = super::checks_value(&[("shape", "pass")]);
    let entry = super::record("check", alloc::vec![super::string("shape"), super::string("pass")]);
    assert_eq!(checks, super::record("checks", alloc::vec![super::sequence(alloc::vec![entry])]));
    let refs = super::sequence(alloc::vec![super::string("a"), super::string("b")]);
    assert_eq!(super::refs_sequence(&["a", "b"]), refs);
}

/// What the view readers return is what the builders were given.
// r[verify molten.preserves_rail_toolkit.parser_builders]
#[hegel::test(test_cases = 64)]
fn hegel_built_refs_and_checks_read_back_unchanged(tc: hegel::TestCase) {
    let count = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(4));
    let refs: alloc::vec::Vec<alloc::string::String> = (0..count)
        .map(|_| super::content_ref_from_bytes(&tc.draw(hegel::generators::binary().max_size(16))))
        .collect();
    let wrapped = super::record("refs", alloc::vec![super::refs_sequence(&refs)]);
    assert_eq!(crate::view::record_content_ref_strings(&wrapped, "refs", "refs", 4), Ok(refs.clone()));

    let names: alloc::vec::Vec<alloc::string::String> =
        (0..count).map(|index| alloc::format!("check-{index}")).collect();
    let statuses = ["pass", "fail", "deny"].into_iter().cycle();
    let pairs: alloc::vec::Vec<(&str, &str)> = names.iter().map(alloc::string::String::as_str).zip(statuses).collect();
    let parsed = crate::view::parse_checks_record(&super::checks_value(&pairs), 4, "gate").expect("built checks parse");
    let read: alloc::vec::Vec<(&str, &str)> =
        parsed.iter().map(|check| (check.name.as_str(), check.status.as_str())).collect();
    assert_eq!(read, pairs);
}
