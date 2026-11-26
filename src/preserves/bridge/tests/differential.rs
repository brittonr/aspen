//! Differential properties: the `molten-preserves-core` kernel against `preserves::PackedWriter`
//! and the `preserves` packed reader, across every value class including big integers, IEEE 754
//! bit patterns, sets, dictionaries, `Embedded` values, and annotations.

const MAX_LEAVES: usize = 6;
const MAX_COMBINATIONS: usize = 8;
const MAX_TAKE: usize = 3;
const MAX_TEXT: usize = 12;
const BIG_MIN_BYTES: usize = 17;
const BIG_MAX_BYTES: usize = 24;

/// Bytes from `preserves::PackedWriter`, the reference encoder.
fn packed_writer_bytes(value: &super::IoValue, write_annotations: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = preserves::PackedWriter::new(&mut bytes).set_write_annotations(write_annotations);
    value
        .write(&mut writer, &mut preserves::IOValueDomainEncode)
        .expect("PackedWriter writes to memory");
    bytes
}

/// The previous strict decode: the `preserves` reader plus an exact re-encoding comparison. That
/// reader adds atom lengths without overflow checks and panics in debug builds on an out-of-range
/// varint, so the allocation-free shape preflight (which admits every complete value whose lengths
/// fit the input) screens edited bytes first.
fn reference_strict_decode(bytes: &[u8]) -> Option<super::IoValue> {
    molten_preserves_core::bounds::preflight(bytes, u64::MAX).ok()?;
    let value = preserves::read_iovalue_packed(bytes, false).ok()?;
    (packed_writer_bytes(&value, false) == bytes).then_some(value)
}

fn draw_index(tc: &hegel::TestCase, max: usize) -> usize {
    tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max))
}

/// A minimal integer wider than 128 bits, built by the `preserves` reader.
fn draw_big_integer(tc: &hegel::TestCase) -> super::IoValue {
    let mut payload = tc.draw(hegel::generators::binary().min_size(BIG_MIN_BYTES).max_size(BIG_MAX_BYTES));
    // A first byte of neither 0x00 nor 0xff never only repeats the sign, so the payload is minimal.
    payload[0] = if payload[0] & 0x80 == 0 {
        payload[0] | 0x01
    } else {
        payload[0] & 0xfe
    };
    let length = u8::try_from(payload.len()).expect("short payload");
    let bytes = [&[0xb0, length][..], &payload].concat();
    preserves::read_iovalue_packed(&bytes, false).expect("minimal big integer")
}

fn draw_leaf(tc: &hegel::TestCase) -> super::IoValue {
    match draw_index(tc, 8) {
        0 => super::IoValue::new(tc.draw(hegel::generators::booleans())),
        1 => super::IoValue::new(f64::from_bits(tc.draw(hegel::generators::integers::<u64>()))),
        2 => super::IoValue::new(preserves::SignedInteger::from(tc.draw(hegel::generators::integers::<i64>()))),
        3 => super::IoValue::new(preserves::SignedInteger::from(tc.draw(hegel::generators::integers::<u128>()))),
        4 => super::IoValue::new(preserves::SignedInteger::from(tc.draw(hegel::generators::integers::<i128>()))),
        5 => draw_big_integer(tc),
        6 => super::IoValue::new(tc.draw(hegel::generators::text().max_size(MAX_TEXT))),
        7 => super::IoValue::bytes(tc.draw(hegel::generators::binary().max_size(MAX_TEXT))),
        _ => super::IoValue::symbol(tc.draw(hegel::generators::text().max_size(MAX_TEXT))),
    }
}

fn combine(kind: usize, label: super::IoValue, mut items: Vec<super::IoValue>) -> super::IoValue {
    match kind {
        0 => super::IoValue::record(label, items),
        1 => super::IoValue::new(items),
        2 => super::IoValue::new(items.into_iter().collect::<preserves::Set<super::IoValue>>()),
        3 => {
            let values = items.split_off(items.len() / 2);
            super::IoValue::new(items.into_iter().zip(values).collect::<preserves::Map<_, _>>())
        }
        4 => super::IoValue::embedded(label),
        _ => super::IoValue::new(preserves::Annotations::new(label.0, items)),
    }
}

/// A random value built bottom-up from a pool of leaves, so the generator never recurses.
fn draw_value(tc: &hegel::TestCase) -> super::IoValue {
    let mut pool: Vec<super::IoValue> = (0..=draw_index(tc, MAX_LEAVES)).map(|_| draw_leaf(tc)).collect();
    for _ in 0..draw_index(tc, MAX_COMBINATIONS) {
        let kind = draw_index(tc, 5);
        let items = pool.split_off(pool.len().saturating_sub(draw_index(tc, MAX_TAKE)));
        let label = pool.pop().unwrap_or_else(|| draw_leaf(tc));
        pool.push(combine(kind, label, items));
    }
    super::IoValue::new(pool)
}

// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 200)]
fn hegel_kernel_bytes_and_refs_match_preserves_packed_writer(tc: hegel::TestCase) {
    let value = draw_value(&tc);
    let expected = packed_writer_bytes(&value, false);
    let packed = super::value_bridge::packed_from_iovalue(&value).expect("convert to the kernel model");
    assert_eq!(molten_preserves_core::writer::encode_canonical(&packed), expected);
    assert_eq!(super::canonical_bytes(&value).expect("canonical bytes"), expected);
    let expected_ref = format!("blake3:{}", blake3::hash(&expected).to_hex());
    assert_eq!(molten_preserves_core::identity::ContentRef::of_value(&packed).as_str(), expected_ref);
    assert_eq!(super::canonical_hash(&value).expect("canonical hash"), expected_ref);

    let decoded = super::strict_canonical_decode(&expected).expect("kernel strict decode of PackedWriter bytes");
    let reference = reference_strict_decode(&expected).expect("reference strict decode");
    assert_eq!(decoded.value, reference);
    assert_eq!(decoded.value_ref.as_str(), expected_ref);
}

/// `PackedWriter` with annotations sorts set and dictionary chunks by their annotated bytes; the
/// kernel orders by annotation-free canonical bytes. Skipping annotations therefore yields the
/// canonical value, or denies an order that only the annotations produced.
// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 100)]
fn hegel_annotated_writer_bytes_are_rejected_strictly_and_skip_to_the_same_value(tc: hegel::TestCase) {
    let value = draw_value(&tc);
    let canonical = packed_writer_bytes(&value, false);
    let annotated = packed_writer_bytes(&value, true);
    let limits = molten_preserves_core::reader::DecodeLimits::IN_MEMORY;
    match molten_preserves_core::reader::decode_skipping_annotations(&annotated, &limits) {
        Ok(skipped) => assert_eq!(molten_preserves_core::writer::encode_canonical(&skipped), canonical),
        Err(error) => {
            assert_eq!(error, molten_preserves_core::reader::failure::DecodeError::UnsortedElements);
            assert_ne!(annotated, canonical);
        }
    }
    let strict = molten_preserves_core::reader::decode_canonical(&annotated, &limits);
    assert_eq!(strict.is_ok(), annotated == canonical);
}

/// Single-byte edits of canonical bytes: the kernel accepts exactly the inputs the reference
/// strict decode accepts, and both decode them to the same value.
// r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
#[hegel::test(test_cases = 200)]
fn hegel_kernel_strict_decode_agrees_with_the_reference_on_edited_bytes(tc: hegel::TestCase) {
    let mut bytes = packed_writer_bytes(&draw_value(&tc), false);
    let index = draw_index(&tc, bytes.len());
    let byte = tc.draw(hegel::generators::integers::<u8>());
    match draw_index(&tc, 2) {
        0 if index < bytes.len() => bytes[index] = byte,
        1 if index < bytes.len() => {
            bytes.remove(index);
        }
        _ => bytes.insert(index, byte),
    }
    let kernel = super::strict_canonical_decode(&bytes).ok();
    let reference = reference_strict_decode(&bytes);
    assert_eq!(kernel.as_ref().map(|decoded| &decoded.value), reference.as_ref(), "input {bytes:02x?}");
}
