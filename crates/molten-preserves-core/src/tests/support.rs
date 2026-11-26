//! Shared fixtures and a bounded, non-recursive Hegel value generator.

use alloc::string::ToString;

pub(super) fn int(value: i128) -> crate::model::PackedValue {
    crate::model::PackedValue::from(value)
}

pub(super) fn decode(bytes: &[u8]) -> Result<crate::model::PackedValue, crate::reader::failure::DecodeError> {
    crate::reader::decode_canonical(bytes, &crate::reader::DecodeLimits::IN_MEMORY)
}

pub(super) fn encode(value: &crate::model::PackedValue) -> alloc::vec::Vec<u8> {
    crate::writer::encode_canonical(value)
}

/// `<example "a" [1 2 3]>`.
pub(super) fn example_record() -> crate::model::PackedValue {
    crate::model::PackedValue::symbol_record("example", alloc::vec![
        crate::model::PackedValue::string("a"),
        crate::model::PackedValue::Sequence(alloc::vec![int(1), int(2), int(3)]),
    ])
}

const MAX_LEAVES: usize = 6;
const MAX_COMBINATIONS: usize = 8;
const MAX_TAKE: usize = 3;
const MAX_TEXT: usize = 12;

fn draw_index(tc: &hegel::TestCase, max: usize) -> usize {
    tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max))
}

fn draw_leaf(tc: &hegel::TestCase) -> crate::model::PackedValue {
    match draw_index(tc, 7) {
        0 => crate::model::PackedValue::Boolean(tc.draw(hegel::generators::booleans())),
        1 => crate::model::PackedValue::Double(crate::model::PackedDouble::from_bits(
            tc.draw(hegel::generators::integers::<u64>()),
        )),
        2 => crate::model::PackedValue::from(tc.draw(hegel::generators::integers::<i64>())),
        3 => crate::model::PackedValue::from(tc.draw(hegel::generators::integers::<u128>())),
        4 => crate::model::PackedValue::from(tc.draw(hegel::generators::integers::<i128>())),
        5 => crate::model::PackedValue::String(tc.draw(hegel::generators::text().max_size(MAX_TEXT))),
        6 => crate::model::PackedValue::ByteString(tc.draw(hegel::generators::binary().max_size(MAX_TEXT))),
        _ => crate::model::PackedValue::Symbol(tc.draw(hegel::generators::text().max_size(MAX_TEXT))),
    }
}

fn take_tail(
    pool: &mut alloc::vec::Vec<crate::model::PackedValue>,
    count: usize,
) -> alloc::vec::Vec<crate::model::PackedValue> {
    let keep = pool.len().saturating_sub(count);
    pool.split_off(keep)
}

fn combine(
    kind: usize,
    label: crate::model::PackedValue,
    mut items: alloc::vec::Vec<crate::model::PackedValue>,
) -> crate::model::PackedValue {
    match kind {
        0 => crate::model::PackedValue::record(label, items),
        1 => crate::model::PackedValue::Sequence(items),
        2 => crate::model::PackedValue::Set(items.into_iter().collect()),
        3 => {
            let values = items.split_off(items.len() / 2);
            crate::model::PackedValue::Dictionary(items.into_iter().zip(values).collect())
        }
        _ => crate::model::PackedValue::embedded(label),
    }
}

/// A random value across every packed class. The tree is built bottom-up from a pool of leaves,
/// so the generator never recurses and stays small.
pub(super) fn draw_value(tc: &hegel::TestCase) -> crate::model::PackedValue {
    let mut pool: alloc::vec::Vec<crate::model::PackedValue> =
        (0..=draw_index(tc, MAX_LEAVES)).map(|_| draw_leaf(tc)).collect();
    for _ in 0..draw_index(tc, MAX_COMBINATIONS) {
        let kind = draw_index(tc, 4);
        let count = draw_index(tc, MAX_TAKE);
        let items = take_tail(&mut pool, count);
        let label = pool.pop().unwrap_or_else(|| draw_leaf(tc));
        pool.push(combine(kind, label, items));
    }
    crate::model::PackedValue::Sequence(pool)
}

/// A content reference derived from `label`.
pub(super) fn fixture_ref(label: &str) -> crate::identity::ContentRef {
    crate::identity::ContentRef::of_bytes(label.to_string().as_bytes())
}
