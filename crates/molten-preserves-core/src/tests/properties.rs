//! Bounded Hegel properties of the encoder, the strict decoder, identity, and the walker.

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 128)]
fn hegel_canonical_bytes_round_trip_through_the_strict_decoder(tc: hegel::TestCase) {
    let value = super::support::draw_value(&tc);
    let bytes = super::support::encode(&value);
    assert_eq!(super::support::decode(&bytes).as_ref(), Ok(&value));
    assert_eq!(crate::identity::ContentRef::of_value(&value), crate::identity::ContentRef::of_bytes(&bytes));
    assert_eq!(crate::bounds::preflight(&bytes, u64::MAX), Ok(()));
}

/// The strict decoder accepts only fixed points of the encoder: any single-byte edit of a canonical
/// encoding is either rejected or is itself the canonical encoding of the value it decodes to.
// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 128)]
fn hegel_strict_decoder_accepts_only_canonical_encodings(tc: hegel::TestCase) {
    let mut bytes = super::support::encode(&super::support::draw_value(&tc));
    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(bytes.len()));
    let byte = tc.draw(hegel::generators::integers::<u8>());
    match tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(2)) {
        0 if index < bytes.len() => bytes[index] = byte,
        1 if index < bytes.len() => {
            bytes.remove(index);
        }
        _ => bytes.insert(index, byte),
    }
    if let Ok(decoded) = super::support::decode(&bytes) {
        assert_eq!(super::support::encode(&decoded), bytes);
    }
}

#[hegel::test(test_cases = 64)]
fn hegel_collection_bytes_ignore_insertion_order(tc: hegel::TestCase) {
    let count = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(6));
    let values: alloc::vec::Vec<crate::model::PackedValue> =
        (0..count).map(|_| super::support::draw_value(&tc)).collect();
    let forward: crate::model::collections::PackedSet = values.iter().cloned().collect();
    let backward: crate::model::collections::PackedSet = values.iter().rev().cloned().collect();
    assert_eq!(
        super::support::encode(&crate::model::PackedValue::Set(forward.clone())),
        super::support::encode(&crate::model::PackedValue::Set(backward))
    );
    let encoded: alloc::vec::Vec<&[u8]> = forward.encoded().collect();
    assert!(encoded.windows(2).all(|pair| pair[0] < pair[1]));
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 64)]
fn hegel_embedded_markers_never_evade_traversal(tc: hegel::TestCase) {
    let layers = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(6));
    let markers = crate::inspect::catalog::SENSITIVE_STRUCTURAL_MARKERS;
    let marker = markers[tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(markers.len() - 1))];
    let wrapped = (0..layers)
        .fold(crate::model::PackedValue::symbol_record(marker, alloc::vec::Vec::new()), |inner, _| {
            crate::model::PackedValue::embedded(inner)
        });
    let value = crate::model::PackedValue::Sequence(alloc::vec![super::support::draw_value(&tc), wrapped]);
    let decoded = super::support::decode(&super::support::encode(&value)).expect("canonical bytes decode");
    let found = crate::inspect::catalog::find_sensitive(&decoded).expect("traversal within default budget");
    assert!(found.is_some());
}

#[hegel::test(test_cases = 64)]
fn hegel_uppercase_or_bare_refs_never_decode_as_optional_refs(tc: hegel::TestCase) {
    let bytes = tc.draw(hegel::generators::binary().max_size(64));
    let reference = crate::identity::ContentRef::of_bytes(&bytes);
    let some = crate::identity::optional_ref_value(Some(reference.as_str()));
    assert_eq!(crate::identity::decode_optional_ref(&some), Ok(Some(reference.clone())));
    let bare = crate::model::PackedValue::string(reference.as_str());
    assert!(crate::identity::decode_optional_ref(&bare).is_err());
    let uppercase = alloc::format!("blake3:{}", reference.hex().to_ascii_uppercase());
    if uppercase != reference.as_str() {
        let wrapped = crate::identity::optional_ref_value(Some(&uppercase));
        assert!(crate::identity::decode_optional_ref(&wrapped).is_err());
    }
}
