// Timed comparison of the three canonical encoding paths on representative records.
//
// Run explicitly and record the output:
// `cargo test -p molten --release --lib codec::encoding_throughput -- --ignored --nocapture`
// Timings are diagnostic observations of one host, never correctness; the bytes are asserted equal.

#![allow(
    tigerstyle::ambient_clock,
    reason = "the measurement harness records wall-clock latency of one diagnostic run; no decision reads the clock"
)]

const SMALL_ROUNDS: u32 = 20_000;
const LARGE_ROUNDS: u32 = 200;
const COLLECTION_ENTRIES: u32 = 1_024;
const PAYLOAD_BYTES: usize = 4_096;

fn reference(label: &str, index: u32) -> String {
    super::content_ref_from_bytes(format!("{label}-{index}").as_bytes())
}

fn ref_field(label: &'static str, index: u32) -> super::IoValue {
    super::record(label, vec![super::string(reference(label, index))])
}

/// A world-commit-shaped record: schema, refs, an ordered entry list, and checks.
fn world_commit_record() -> super::IoValue {
    let entries = (0..32)
        .map(|index| super::record("entry", vec![super::string(format!("path/{index}")), ref_field("blob", index)]));
    super::record("world-commit-v1", vec![
        super::string("molten.world.commit.v1"),
        ref_field("parent", 0),
        ref_field("tree", 0),
        super::record("entries", vec![super::sequence(entries.collect())]),
        super::record("sequence", vec![super::u64_value(42)]),
        super::checks_value(&[("parent-bound", "pass"), ("tree-bound", "pass"), ("authority", "pass")]),
    ])
}

/// A Raft-envelope-shaped record with a binary command payload.
fn raft_envelope_record() -> super::IoValue {
    super::record("raft-command-envelope-v1", vec![
        super::string("molten.raft.command-envelope.v1"),
        ref_field("group", 1),
        super::record("term", vec![super::u64_value(7)]),
        super::record("index", vec![super::u64_value(1_024)]),
        super::record("payload", vec![super::IoValue::bytes(vec![0x5a; PAYLOAD_BYTES])]),
        ref_field("payload-ref", 1),
    ])
}

/// A large set of refs beside a large dictionary of counters.
fn large_collections_record() -> super::IoValue {
    let set: preserves::Set<super::IoValue> =
        (0..COLLECTION_ENTRIES).map(|index| super::string(reference("member", index))).collect();
    let map: preserves::Map<super::IoValue, super::IoValue> = (0..COLLECTION_ENTRIES)
        .map(|index| (super::string(reference("key", index)), super::u64_value(u64::from(index))))
        .collect();
    super::IoValue::new(vec![super::IoValue::new(set), super::IoValue::new(map)])
}

fn nanos_per_round(rounds: u32, mut encode: impl FnMut() -> Vec<u8>) -> u128 {
    let started = std::time::Instant::now();
    for _ in 0..rounds {
        std::hint::black_box(encode());
    }
    started.elapsed().as_nanos().checked_div(u128::from(rounds)).expect("at least one round")
}

fn measure(label: &str, value: &super::IoValue, rounds: u32) {
    let writer = || preserves::write_iovalue_packed(value, false).expect("PackedWriter bytes");
    let tree = || {
        let packed = super::value_bridge::packed_from_iovalue(value).expect("kernel model");
        molten_preserves_core::writer::encode_canonical(&packed)
    };
    let borrowed = || super::stream_bridge::encode_iovalue(value).expect("borrowed bytes");
    assert_eq!(borrowed(), writer());
    assert_eq!(tree(), writer());
    let writer_ns = nanos_per_round(rounds, writer);
    let tree_ns = nanos_per_round(rounds, tree);
    let borrowed_ns = nanos_per_round(rounds, borrowed);
    eprintln!(
        "{label}: {} bytes; PackedWriter {writer_ns} ns, PackedValue tree {tree_ns} ns, borrowed {borrowed_ns} ns",
        writer().len()
    );
}

#[test]
#[ignore = "diagnostic measurement; run explicitly and record the output"]
fn writer_tree_and_borrowed_paths() {
    measure("world-commit", &world_commit_record(), SMALL_ROUNDS);
    measure("raft-envelope", &raft_envelope_record(), SMALL_ROUNDS);
    measure("large-set-and-dictionary", &large_collections_record(), LARGE_ROUNDS);
}
