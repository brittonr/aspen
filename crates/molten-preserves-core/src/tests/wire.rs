//! Canonical wire vectors: exact encoder bytes and strict decoder round trips.

fn int(value: i128) -> crate::model::PackedValue {
    super::support::int(value)
}

fn assert_round_trip(value: &crate::model::PackedValue, expected: &[u8]) {
    assert_eq!(super::support::encode(value), expected);
    assert_eq!(super::support::decode(expected).as_ref(), Ok(value));
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn records_strings_and_sequences_use_packed_tags() {
    let expected = b"\xb4\xb3\x07example\xb1\x01a\xb5\xb0\x01\x01\xb0\x01\x02\xb0\x01\x03\x84\x84";
    assert_round_trip(&super::support::example_record(), expected);
    assert_round_trip(&crate::model::PackedValue::Boolean(false), b"\x80");
    assert_round_trip(&crate::model::PackedValue::Boolean(true), b"\x81");
    assert_round_trip(&crate::model::PackedValue::ByteString(alloc::vec![1, 2]), b"\xb2\x02\x01\x02");
    let embedded = crate::model::PackedValue::embedded(crate::model::PackedValue::string("x"));
    assert_round_trip(&embedded, b"\x86\xb1\x01x");
}

#[test]
fn integers_use_minimal_twos_complement() {
    assert_round_trip(&int(0), b"\xb0\x00");
    assert_round_trip(&int(-1), b"\xb0\x01\xff");
    assert_round_trip(&int(127), b"\xb0\x01\x7f");
    assert_round_trip(&int(128), b"\xb0\x02\x00\x80");
    assert_round_trip(&int(-128), b"\xb0\x01\x80");
    assert_round_trip(&int(-129), b"\xb0\x02\xff\x7f");
    assert_round_trip(&int(255), b"\xb0\x02\x00\xff");
    let minimum = b"\xb0\x10\x80\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    assert_round_trip(&int(i128::MIN), minimum);
    let maximum = b"\xb0\x11\x00\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    assert_round_trip(&crate::model::PackedValue::from(u128::MAX), maximum);
}

#[test]
fn integers_convert_back_within_their_range() {
    let wide = crate::model::signed::PackedInteger::from(u128::MAX);
    assert_eq!(wide.to_u128(), Some(u128::MAX));
    assert_eq!(wide.to_i128(), None);
    let negative = crate::model::signed::PackedInteger::from(-5_i128);
    assert_eq!(negative.to_i128(), Some(-5));
    assert_eq!(negative.to_u128(), None);
    assert_eq!(crate::model::signed::PackedInteger::from(42_i128).to_u64(), Some(42));
}

#[test]
fn doubles_keep_their_exact_bits() {
    assert_round_trip(&crate::model::PackedValue::from(1.0_f64), b"\x87\x08\x3f\xf0\x00\x00\x00\x00\x00\x00");
    let negative_zero = crate::model::PackedValue::from(-0.0_f64);
    assert_round_trip(&negative_zero, b"\x87\x08\x80\x00\x00\x00\x00\x00\x00\x00");
    let nan = crate::model::PackedDouble::from_bits(0x7ff8_0000_0000_0001);
    assert_round_trip(&crate::model::PackedValue::Double(nan), b"\x87\x08\x7f\xf8\x00\x00\x00\x00\x00\x01");
}

#[test]
fn lengths_use_minimal_varints() {
    let text = "z".repeat(200);
    let expected = [&b"\xb1\xc8\x01"[..], text.as_bytes()].concat();
    assert_round_trip(&crate::model::PackedValue::string(&text), &expected);
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn sets_and_dictionaries_follow_canonical_byte_order() {
    let numbers: crate::model::collections::PackedSet = [int(2), int(1)].into_iter().collect();
    assert_round_trip(&crate::model::PackedValue::Set(numbers), b"\xb6\xb0\x01\x01\xb0\x01\x02\x84");
    // Byte order, not value order: the shorter string sorts first by its length byte.
    let strings: crate::model::collections::PackedSet =
        ["aa", "b"].into_iter().map(crate::model::PackedValue::string).collect();
    assert_round_trip(&crate::model::PackedValue::Set(strings), b"\xb6\xb1\x01b\xb1\x02aa\x84");
    let dictionary: crate::model::collections::PackedDictionary = [
        (crate::model::PackedValue::symbol("b"), int(1)),
        (crate::model::PackedValue::symbol("a"), int(2)),
    ]
    .into_iter()
    .collect();
    assert_round_trip(
        &crate::model::PackedValue::Dictionary(dictionary),
        b"\xb7\xb3\x01a\xb0\x01\x02\xb3\x01b\xb0\x01\x01\x84",
    );
}

#[test]
fn collections_deduplicate_by_canonical_bytes() {
    let mut set = crate::model::collections::PackedSet::new();
    assert!(set.insert(int(7)));
    assert!(!set.insert(int(7)));
    assert!(set.contains(&int(7)));
    assert_eq!(set.iter().count(), 1);
    let mut dictionary = crate::model::collections::PackedDictionary::new();
    assert_eq!(dictionary.insert(int(1), int(2)), None);
    assert_eq!(dictionary.insert(int(1), int(3)), Some(int(2)));
    assert_eq!(dictionary.get(&int(1)), Some(&int(3)));
}

#[test]
fn skipping_annotations_yields_the_unannotated_value() {
    let limits = crate::reader::DecodeLimits::IN_MEMORY;
    let annotated = b"\xb5\x85\xb1\x01n\xb0\x01\x01\x84";
    let decoded = crate::reader::decode_skipping_annotations(annotated, &limits);
    assert_eq!(decoded, Ok(crate::model::PackedValue::Sequence(alloc::vec![int(1)])));
    let missing = crate::reader::decode_skipping_annotations(b"\xb5\x85\xb1\x01n\x84", &limits);
    assert_eq!(missing, Err(crate::reader::failure::DecodeError::MissingValue));
}

/// Annotated set elements and dictionary keys are ordered by their annotation-free canonical
/// bytes, not by their annotated input spans.
// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn skipped_annotations_do_not_change_collection_order() {
    let limits = crate::reader::DecodeLimits::IN_MEMORY;
    let annotated_two_first = b"\xb6\x85\xb1\x01n\xb0\x01\x02\xb0\x01\x01\x84";
    assert_eq!(
        crate::reader::decode_skipping_annotations(annotated_two_first, &limits),
        Err(crate::reader::failure::DecodeError::UnsortedElements)
    );
    let canonical_with_annotated_two = b"\xb6\xb0\x01\x01\x85\xb1\x01n\xb0\x01\x02\x84";
    let expected: crate::model::collections::PackedSet = [int(1), int(2)].into_iter().collect();
    assert_eq!(
        crate::reader::decode_skipping_annotations(canonical_with_annotated_two, &limits),
        Ok(crate::model::PackedValue::Set(expected))
    );
    let dictionary = b"\xb7\xb0\x01\x01\x80\x85\xb1\x01n\xb0\x01\x02\x81\x84";
    assert!(crate::reader::decode_skipping_annotations(dictionary, &limits).is_ok());
    let unsorted = b"\xb7\x85\xb1\x01n\xb0\x01\x02\x81\xb0\x01\x01\x80\x84";
    assert_eq!(
        crate::reader::decode_skipping_annotations(unsorted, &limits),
        Err(crate::reader::failure::DecodeError::UnsortedElements)
    );
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn incremental_writer_matches_the_tree_encoder() {
    let mut writer = crate::incremental::CanonicalWriter::new();
    writer.open_record();
    writer.symbol("example");
    writer.open_set();
    writer.unsigned(u128::MAX);
    writer.next_item();
    writer.string("aa");
    writer.next_item();
    writer.signed(-129);
    writer.next_item();
    writer.close_sorted();
    writer.open_dictionary();
    writer.symbol("b");
    writer.boolean(true);
    writer.next_item();
    writer.symbol("a");
    writer.double(1.0_f64.to_bits());
    writer.next_item();
    writer.close_sorted();
    writer.embedded();
    writer.byte_string(&[1, 2]);
    writer.close();
    let set: crate::model::collections::PackedSet = [
        crate::model::PackedValue::from(u128::MAX),
        crate::model::PackedValue::string("aa"),
        int(-129),
    ]
    .into_iter()
    .collect();
    let dictionary: crate::model::collections::PackedDictionary = [
        (crate::model::PackedValue::symbol("b"), crate::model::PackedValue::Boolean(true)),
        (crate::model::PackedValue::symbol("a"), crate::model::PackedValue::from(1.0_f64)),
    ]
    .into_iter()
    .collect();
    let expected = crate::model::PackedValue::symbol_record("example", alloc::vec![
        crate::model::PackedValue::Set(set),
        crate::model::PackedValue::Dictionary(dictionary),
        crate::model::PackedValue::embedded(crate::model::PackedValue::ByteString(alloc::vec![1, 2])),
    ]);
    assert_eq!(writer.finish(), Some(super::support::encode(&expected)));
}

#[test]
fn deep_values_encode_without_recursion() {
    const DEPTH: usize = 5_000;
    const ADMITTED: usize = 1_000;
    let value = (0..DEPTH).fold(int(0), |inner, _| crate::model::PackedValue::embedded(inner));
    let bytes = super::support::encode(&value);
    assert_eq!(bytes.len(), DEPTH + 2);
    assert_eq!(
        super::support::decode(&bytes),
        Err(crate::reader::failure::DecodeError::DepthExceeded {
            limit: crate::reader::IN_MEMORY_MAX_DEPTH
        })
    );
    assert!(super::support::decode(&bytes[DEPTH - ADMITTED..]).is_ok());
}
