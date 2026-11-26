//! Canonical identity: packed Preserves bytes and domain-separated BLAKE3 refs.

const MAX_DEPTH: u8 = 3;
const MAX_ITEMS: usize = 4;
const MAX_ATOM_BYTES: usize = 24;
const LEAF_KINDS: u8 = 4;
const ALL_KINDS: u8 = 6;
const TAG_END: u8 = 0x84;
const TAG_SIGNED_INTEGER: u8 = 0xb0;
const TAG_STRING: u8 = 0xb1;
const TAG_BYTE_STRING: u8 = 0xb2;
const TAG_SYMBOL: u8 = 0xb3;
const TAG_RECORD: u8 = 0xb4;
const TAG_SEQUENCE: u8 = 0xb5;
const VARINT_PAYLOAD_BITS: u32 = 7;
const VARINT_CONTINUATION: u8 = 0x80;
const VARINT_PAYLOAD_MASK: u8 = 0x7f;
const MAX_INTEGER_BYTES: usize = 16;
const FIRST_DOMAIN: &str = "molten.test.hegel-identity.first.v1";
const SECOND_DOMAIN: &str = "molten.test.hegel-identity.second.v1";

fn draw_text(tc: &hegel::TestCase) -> String {
    let bytes = tc.draw(
        hegel::generators::vecs(hegel::generators::integers::<u8>().min_value(0x20).max_value(0x7e))
            .max_size(MAX_ATOM_BYTES),
    );
    let mut text = bytes.into_iter().map(char::from).collect::<String>();
    if tc.draw(hegel::generators::booleans()) {
        // Multi-byte scalar values exercise byte-length (not char-count) framing.
        text.push('\u{00e9}');
        text.push('\u{1f980}');
    }
    text
}

fn draw_integer(tc: &hegel::TestCase) -> i128 {
    let high = tc.draw(hegel::generators::integers::<i64>());
    let low = tc.draw(hegel::generators::integers::<u64>());
    if tc.draw(hegel::generators::booleans()) {
        (i128::from(high) << 64) | i128::from(low)
    } else {
        i128::from(high)
    }
}

fn draw_value(tc: &hegel::TestCase, depth: u8) -> molten_core::protocol_oracle::CanonicalValue {
    let kinds = if depth >= MAX_DEPTH { LEAF_KINDS } else { ALL_KINDS };
    match tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(kinds - 1)) {
        0 => molten_core::protocol_oracle::CanonicalValue::Integer(draw_integer(tc)),
        1 => molten_core::protocol_oracle::CanonicalValue::String(draw_text(tc)),
        2 => molten_core::protocol_oracle::CanonicalValue::Bytes(
            tc.draw(hegel::generators::binary().max_size(MAX_ATOM_BYTES)),
        ),
        3 => molten_core::protocol_oracle::CanonicalValue::Symbol(draw_text(tc)),
        4 => {
            let count = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(MAX_ITEMS));
            let fields = (0..count).map(|_| draw_value(tc, depth + 1)).collect();
            molten_core::protocol_oracle::CanonicalValue::Record {
                label: draw_text(tc),
                fields,
            }
        }
        _ => {
            let count = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(MAX_ITEMS));
            molten_core::protocol_oracle::CanonicalValue::Sequence(
                (0..count).map(|_| draw_value(tc, depth + 1)).collect(),
            )
        }
    }
}

/// Strict reference decoder: rejects non-minimal integers, non-minimal length varints, invalid
/// UTF-8, non-symbol record labels, unknown tags, and trailing bytes.
fn decode(bytes: &[u8]) -> Option<molten_core::protocol_oracle::CanonicalValue> {
    let mut cursor = 0;
    let value = decode_value(bytes, &mut cursor)?;
    (cursor == bytes.len()).then_some(value)
}

fn decode_value(bytes: &[u8], cursor: &mut usize) -> Option<molten_core::protocol_oracle::CanonicalValue> {
    let tag = *bytes.get(*cursor)?;
    *cursor += 1;
    match tag {
        TAG_SIGNED_INTEGER => {
            decode_integer(decode_atom(bytes, cursor)?).map(molten_core::protocol_oracle::CanonicalValue::Integer)
        }
        TAG_STRING => {
            decode_text(decode_atom(bytes, cursor)?).map(molten_core::protocol_oracle::CanonicalValue::String)
        }
        TAG_BYTE_STRING => decode_atom(bytes, cursor)
            .map(|payload| molten_core::protocol_oracle::CanonicalValue::Bytes(payload.to_vec())),
        TAG_SYMBOL => {
            decode_text(decode_atom(bytes, cursor)?).map(molten_core::protocol_oracle::CanonicalValue::Symbol)
        }
        TAG_RECORD => {
            if *bytes.get(*cursor)? != TAG_SYMBOL {
                return None;
            }
            *cursor += 1;
            let label = decode_text(decode_atom(bytes, cursor)?)?;
            let fields = decode_until_end(bytes, cursor)?;
            Some(molten_core::protocol_oracle::CanonicalValue::Record { label, fields })
        }
        TAG_SEQUENCE => decode_until_end(bytes, cursor).map(molten_core::protocol_oracle::CanonicalValue::Sequence),
        _ => None,
    }
}

fn decode_until_end(bytes: &[u8], cursor: &mut usize) -> Option<Vec<molten_core::protocol_oracle::CanonicalValue>> {
    // Every item consumes at least one byte, so the remaining input bounds the item count.
    let mut items = Vec::with_capacity(bytes.len().saturating_sub(*cursor));
    while *bytes.get(*cursor)? != TAG_END {
        items.push(decode_value(bytes, cursor)?);
    }
    *cursor += 1;
    Some(items)
}

fn decode_atom<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let mut length = 0_usize;
    let mut shift = 0_u32;
    loop {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        let payload = usize::from(byte & VARINT_PAYLOAD_MASK);
        length |= payload.checked_shl(shift)?;
        if byte & VARINT_CONTINUATION == 0 {
            // A zero final group after other groups is a non-minimal varint.
            if shift > 0 && payload == 0 {
                return None;
            }
            break;
        }
        shift += VARINT_PAYLOAD_BITS;
    }
    let end = cursor.checked_add(length)?;
    let payload = bytes.get(*cursor..end)?;
    *cursor = end;
    Some(payload)
}

fn decode_text(payload: &[u8]) -> Option<String> {
    String::from_utf8(payload.to_vec()).ok()
}

fn decode_integer(payload: &[u8]) -> Option<i128> {
    if payload.len() > MAX_INTEGER_BYTES {
        return None;
    }
    let (Some(first), second) = (payload.first(), payload.get(1)) else {
        return Some(0);
    };
    if let Some(second) = second {
        let is_redundant_zero = *first == 0x00 && second & 0x80 == 0;
        let is_redundant_sign = *first == 0xff && second & 0x80 != 0;
        if is_redundant_zero || is_redundant_sign {
            return None;
        }
    } else if *first == 0x00 {
        // Zero is the empty payload, never a single zero byte.
        return None;
    }
    let fill = if first & 0x80 == 0 { 0x00 } else { 0xff };
    let mut full = [fill; MAX_INTEGER_BYTES];
    full[MAX_INTEGER_BYTES - payload.len()..].copy_from_slice(payload);
    Some(i128::from_be_bytes(full))
}

// Encoding is a deterministic, injective, strictly canonical function of the value: the strict
// decoder recovers exactly the drawn value, so two distinct values can never share bytes or refs.
#[hegel::test(test_cases = 200, derandomize = true, database = None)]
fn hegel_canonical_encoding_round_trips_through_a_strict_decoder(tc: hegel::TestCase) {
    let value = draw_value(&tc, 0);
    let bytes = molten_core::protocol_oracle::encode_canonical(&value);
    assert_eq!(molten_core::protocol_oracle::encode_canonical(&value), bytes, "encoding must be deterministic");
    assert_eq!(decode(&bytes), Some(value.clone()), "strict decode must recover the value");

    let other = draw_value(&tc, 0);
    let other_bytes = molten_core::protocol_oracle::encode_canonical(&other);
    assert_eq!(value == other, bytes == other_bytes, "byte identity must coincide with value identity");
    assert_eq!(
        value == other,
        molten_core::protocol_oracle::domain_ref(FIRST_DOMAIN, &bytes)
            == molten_core::protocol_oracle::domain_ref(FIRST_DOMAIN, &other_bytes),
        "ref identity must coincide with value identity"
    );
}

// Integers use the shortest two's-complement payload, so numerically equal values have one
// encoding.
#[hegel::test(test_cases = 200, derandomize = true, database = None)]
fn hegel_integer_payloads_are_minimal_and_exact(tc: hegel::TestCase) {
    let integer = draw_integer(&tc);
    let bytes =
        molten_core::protocol_oracle::encode_canonical(&molten_core::protocol_oracle::CanonicalValue::Integer(integer));
    assert_eq!(bytes.first(), Some(&TAG_SIGNED_INTEGER));
    let mut cursor = 1;
    let payload = decode_atom(&bytes, &mut cursor);
    assert_eq!(cursor, bytes.len());
    assert_eq!(payload.map(<[u8]>::is_empty), Some(integer == 0), "zero alone has the empty payload");
    assert_eq!(payload.and_then(decode_integer), Some(integer), "payload must be minimal and exact");
}

// Refs are well formed, stable, and separated by domain: the same bytes under two domains never
// collide.
#[hegel::test(test_cases = 100, derandomize = true, database = None)]
fn hegel_domain_refs_are_stable_wellformed_and_domain_separated(tc: hegel::TestCase) {
    let bytes = molten_core::protocol_oracle::encode_canonical(&draw_value(&tc, 0));
    let first = molten_core::protocol_oracle::domain_ref(FIRST_DOMAIN, &bytes);
    assert!(
        molten_core::protocol_oracle::is_content_ref(&first),
        "ref must be blake3 with 64 lowercase hex digits: {first}"
    );
    assert_eq!(molten_core::protocol_oracle::domain_ref(FIRST_DOMAIN, &bytes), first, "refs must be stable");
    assert_ne!(
        molten_core::protocol_oracle::domain_ref(SECOND_DOMAIN, &bytes),
        first,
        "distinct domains must not share refs"
    );
    assert!(
        !molten_core::protocol_oracle::is_content_ref(&first.to_uppercase()),
        "uppercase refs are not canonical"
    );
}
