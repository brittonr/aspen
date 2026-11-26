//! Negative vectors: non-canonical, malformed, and over-budget input fails closed in the strict
//! decoder and the shape preflight.

type Failure = crate::reader::failure::DecodeError;

fn assert_denied(bytes: &[u8], expected: Failure) {
    assert_eq!(super::support::decode(bytes), Err(expected), "input {bytes:02x?}");
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn non_canonical_encodings_are_rejected() {
    assert_denied(b"\xb1\x80\x00", Failure::NonCanonicalLength);
    assert_denied(b"\xb0\x01\x00", Failure::NonCanonicalInteger);
    assert_denied(b"\xb0\x02\x00\x01", Failure::NonCanonicalInteger);
    assert_denied(b"\xb0\x02\xff\xff", Failure::NonCanonicalInteger);
    assert_denied(b"\x85\xb0\x01\x02\xb0\x01\x01", Failure::Annotation);
    assert_denied(b"\x80\x00", Failure::TrailingBytes);
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn set_and_dictionary_order_and_uniqueness_are_enforced() {
    assert_denied(b"\xb6\xb0\x01\x02\xb0\x01\x01\x84", Failure::UnsortedElements);
    assert_denied(b"\xb6\xb0\x01\x01\xb0\x01\x01\x84", Failure::DuplicateElement);
    assert_denied(b"\xb6\xb1\x02aa\xb1\x01b\x84", Failure::UnsortedElements);
    assert_denied(b"\xb7\xb3\x01b\xb0\x01\x01\xb3\x01a\xb0\x01\x02\x84", Failure::UnsortedElements);
    assert_denied(b"\xb7\xb3\x01a\xb0\x01\x01\xb3\x01a\xb0\x01\x02\x84", Failure::DuplicateElement);
    assert_denied(b"\xb7\xb3\x01a\x84", Failure::MissingValue);
}

#[test]
fn malformed_input_is_rejected() {
    assert_denied(b"", Failure::Truncated);
    assert_denied(b"\xb1\x05a", Failure::Truncated);
    assert_denied(b"\xb5\xb0\x01\x01", Failure::Truncated);
    assert_denied(b"\x82", Failure::InvalidTag(0x82));
    assert_denied(b"\xb1\x01\xff", Failure::InvalidUtf8);
    assert_denied(b"\xb3\x02\xc3\x28", Failure::InvalidUtf8);
    assert_denied(b"\x87\x04\x3f\x80\x00\x00", Failure::InvalidFloatWidth);
    assert_denied(b"\xb4\x84", Failure::MissingRecordLabel);
    assert_denied(b"\x84", Failure::UnexpectedEnd);
    assert_denied(b"\xb5\x86\x84\x84", Failure::MissingValue);
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn oversized_lengths_fail_before_allocation() {
    let maximal = b"\xb2\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01";
    assert_denied(maximal, Failure::LengthOverflow);
    let past_u64 = b"\xb2\xff\xff\xff\xff\xff\xff\xff\xff\xff\x02";
    assert_denied(past_u64, Failure::LengthOverflow);
    assert_denied(b"\xb2\xff\xff\xff\x7f", Failure::Truncated);
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[test]
fn decode_limits_fail_closed_one_past_each_bound() {
    let nested = b"\xb5\xb5\xb5\xb5\x84\x84\x84\x84";
    let depth = |max_depth| crate::reader::DecodeLimits {
        max_depth,
        ..crate::reader::DecodeLimits::IN_MEMORY
    };
    assert!(crate::reader::decode_canonical(nested, &depth(4)).is_ok());
    assert_eq!(crate::reader::decode_canonical(nested, &depth(3)), Err(Failure::DepthExceeded { limit: 3 }));

    let three = b"\xb5\xb0\x01\x01\xb0\x01\x02\xb0\x01\x03\x84";
    let nodes = crate::reader::DecodeLimits {
        max_nodes: 3,
        ..crate::reader::DecodeLimits::IN_MEMORY
    };
    assert_eq!(crate::reader::decode_canonical(three, &nodes), Err(Failure::NodesExceeded { limit: 3 }));
    let width = crate::reader::DecodeLimits {
        max_container_items: 2,
        ..crate::reader::DecodeLimits::IN_MEMORY
    };
    assert_eq!(crate::reader::decode_canonical(three, &width), Err(Failure::ContainerExceeded { limit: 2 }));
    let bytes = crate::reader::DecodeLimits {
        max_bytes: 10,
        ..crate::reader::DecodeLimits::IN_MEMORY
    };
    assert_eq!(crate::reader::decode_canonical(three, &bytes), Err(Failure::InputTooLarge { limit: 10 }));
}

#[test]
fn failures_are_classified() {
    assert_eq!(Failure::Annotation.class(), crate::reader::failure::DecodeErrorClass::NonCanonical);
    assert_eq!(Failure::UnsortedElements.class(), crate::reader::failure::DecodeErrorClass::NonCanonical);
    assert_eq!(Failure::InvalidUtf8.class(), crate::reader::failure::DecodeErrorClass::Malformed);
    assert_eq!(Failure::DepthExceeded { limit: 1 }.class(), crate::reader::failure::DecodeErrorClass::Limit);
}

fn nested_sequences(depth: usize) -> alloc::vec::Vec<u8> {
    let mut bytes = alloc::vec![crate::tags::SEQUENCE; depth];
    bytes.extend(core::iter::repeat_n(crate::tags::END, depth));
    bytes
}

#[test]
fn preflight_admits_complete_values_with_prefixes() {
    let canonical = super::support::encode(&super::support::example_record());
    assert_eq!(crate::bounds::preflight(&canonical, crate::bounds::MAX_VALUE_TOKENS), Ok(()));
    // An annotation and an embedded prefix each wrap one following value; non-canonical lengths are
    // left to the strict decoder.
    assert_eq!(crate::bounds::preflight(b"\x85\xb1\x01a\x86\xb1\x80\x00", 8), Ok(()));
    assert_eq!(crate::bounds::preflight(&nested_sequences(crate::bounds::MAX_DEPTH), 1_000), Ok(()));
}

#[test]
fn preflight_denies_depth_tokens_trailing_and_malformed_input() {
    let denied = Err(crate::bounds::PreflightDenied);
    assert_eq!(crate::bounds::preflight(&nested_sequences(crate::bounds::MAX_DEPTH + 1), 1_000), denied);
    let one_item = b"\xb5\xb0\x01\x01\x84";
    assert_eq!(crate::bounds::preflight(one_item, 3), Ok(()));
    assert_eq!(crate::bounds::preflight(one_item, 2), denied);
    for input in [
        &b"\x80\x80"[..],
        b"",
        b"\x84",
        b"\x82",
        b"\xb1\x05a",
        b"\xb5",
        b"\x86\x84",
        b"\xb2\xff\xff\xff\xff\xff\xff\xff\xff\xff\x02",
    ] {
        assert_eq!(crate::bounds::preflight(input, 8), denied, "input {input:02x?}");
    }
}
