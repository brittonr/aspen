// Included unchanged in each isolated source-copy variant. No product calls are mocked.
use std::fmt::Write;

const CASES: &[(&str, usize, &str, &str, usize, bool)] = &[
    ("representative-sequential", 256, "varied", "insert", 64, true),
    ("representative-random", 256, "random", "shuffle", 64, true),
    ("representative-mixed", 256, "varied", "mixed", 64, true),
    ("representative-prefix", 256, "prefix", "mixed", 64, true),
    ("maximum-sequential", 4096, "varied", "insert", 1024, true),
    ("maximum-random", 4096, "random", "shuffle", 1024, true),
    ("maximum-mixed", 4096, "varied", "mixed", 1024, true),
    ("maximum-prefix", 4096, "prefix", "mixed", 1024, true),
    ("empty", 0, "varied", "none", 0, false),
    ("single", 1, "varied", "mixed", 0, false),
    ("small", 16, "random", "mixed", 16, false),
    ("key-lengths", 64, "lengths", "mixed", 64, false),
    ("value-lengths", 1025, "varied", "mixed", usize::MAX, false),
    ("all-update", 4096, "prefix", "update", 1024, false),
    ("repeated-edits", 1, "varied", "repeat", 64, false),
    ("insert-existing", 1, "varied", "existing", 64, false),
    ("update-missing", 1, "varied", "missing-update", 64, false),
    ("delete-missing", 1, "varied", "missing-delete", 64, false),
    ("error-precedence", 1, "varied", "precedence", 64, false),
    ("empty-key-denied", 0, "varied", "empty-key", 64, false),
    ("oversize-key-denied", 0, "varied", "oversize-key", 64, false),
    ("oversize-value-denied", 0, "varied", "oversize-value", 64, false),
    ("edit-count-denied", 0, "varied", "edit-count", 64, false),
    ("corrupt-block-denied", 16, "varied", "corrupt-block", 64, false),
    ("wrong-count-denied", 16, "varied", "wrong-count", 64, false),
    ("duplicate-block-denied", 16, "varied", "duplicate-block", 64, false),
];
struct Fixture {
    profile: ProllyProfile,
    prior: MapSnapshot,
    prior_entries: Vec<SemanticEntry>,
    edits: Vec<MapEdit>,
}
fn random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
fn new_entry(index: usize, value_length: usize) -> SemanticEntry {
    let mut key = vec![0xfe; 64];
    key[60..].copy_from_slice(&(index as u32).to_be_bytes());
    SemanticEntry { key, value: vec![0xa5; value_length] }
}
fn entries(n: usize, shape: &str, value_length: usize) -> Vec<SemanticEntry> {
    let mut seed = 0x03_bbee_2026_1116_u64;
    let mut entries = (0..n).map(|index| {
        let key = match shape {
            "lengths" => vec![0x80; index + 1],
            "prefix" => {
                let mut key = vec![0x61; 64];
                key[60..].copy_from_slice(&(index as u32).to_be_bytes());
                key
            }
            "random" => {
                let mut key = vec![0; 64];
                for bytes in key.chunks_mut(8) { bytes.copy_from_slice(&random(&mut seed).to_be_bytes()); }
                key[0] = 0x13;
                key[60..].copy_from_slice(&(index as u32).to_be_bytes());
                key
            }
            "varied" => {
                let mut key = vec![0; 4 + index % 61];
                key[..4].copy_from_slice(&(index as u32).to_be_bytes());
                for (offset, byte) in key[4..].iter_mut().enumerate() { *byte = (index ^ offset) as u8; }
                key
            }
            _ => unreachable!("unknown shape"),
        };
        let length = if value_length == usize::MAX { index % 1025 } else { value_length };
        SemanticEntry { key, value: (0..length).map(|offset| (offset ^ index) as u8).collect() }
    }).collect::<Vec<_>>();
    entries.sort();
    assert_eq!(entries.len(), n);
    assert!(entries.windows(2).all(|pair| pair[0].key < pair[1].key));
    entries
}
fn changed(entry: &SemanticEntry) -> SemanticEntry {
    let mut entry = entry.clone();
    if entry.value.is_empty() { entry.value.push(0x80); } else { entry.value[0] ^= 0xff; }
    entry
}
pub fn case_names() -> Vec<&'static str> { CASES.iter().map(|case| case.0).collect() }
fn fixture(name: &str) -> Fixture {
    let &(_, n, shape, mode, value_length, _) = CASES.iter().find(|case| case.0 == name).expect("known fixture");
    let profile = standard_prolly_profile().expect("unchanged standard profile");
    let all = entries(n, shape, value_length);
    let (prior_entries, mut edits) = match mode {
        "insert" | "shuffle" => (Vec::new(), all.into_iter().map(MapEdit::Insert).collect::<Vec<_>>()),
        _ => (all, Vec::new()),
    };
    if mode == "shuffle" {
        let mut seed = 0x03_2026_1116_u64;
        for index in (1..edits.len()).rev() { edits.swap(index, (random(&mut seed) as usize) % (index + 1)); }
    }
    match mode {
        "mixed" => {
            for (index, entry) in prior_entries.iter().enumerate() {
                edits.push(match index % 3 {
                    0 => MapEdit::Update(changed(entry)),
                    1 => MapEdit::Delete(entry.key.clone()),
                    _ => MapEdit::Insert(new_entry(index, entry.value.len())),
                });
            }
        }
        "update" => edits.extend(prior_entries.iter().map(|entry| MapEdit::Update(changed(entry)))),
        "repeat" => {
            let entry = &prior_entries[0];
            edits = vec![MapEdit::Update(changed(entry)), MapEdit::Delete(entry.key.clone()),
                         MapEdit::Insert(entry.clone()), MapEdit::Update(changed(entry))];
        }
        "existing" => edits.push(MapEdit::Insert(prior_entries[0].clone())),
        "missing-update" => edits.push(MapEdit::Update(new_entry(0, 64))),
        "missing-delete" => edits.push(MapEdit::Delete(new_entry(0, 0).key)),
        "precedence" => edits = vec![MapEdit::Delete(prior_entries[0].key.clone()),
                                     MapEdit::Update(prior_entries[0].clone()), MapEdit::Delete(new_entry(0,0).key)],
        "empty-key" => edits.push(MapEdit::Insert(SemanticEntry { key: vec![], value: vec![] })),
        "oversize-key" => edits.push(MapEdit::Insert(SemanticEntry { key: vec![0;65], value: vec![] })),
        "oversize-value" => edits.push(MapEdit::Insert(SemanticEntry { key: vec![0], value: vec![0;1025] })),
        "edit-count" => edits = (0..4097).map(|i| MapEdit::Insert(new_entry(i, 1))).collect(),
        _ => (),
    }
    let mut prior = build_map(&profile, &prior_entries).expect("valid initial snapshot").snapshot;
    match mode {
        "corrupt-block" => prior.blocks[0].bytes[0] ^= 1,
        "wrong-count" => prior.root.entry_count += 1,
        "duplicate-block" => prior.blocks.push(prior.blocks[0].clone()),
        _ => (),
    }
    Fixture { profile, prior, prior_entries, edits }
}
// Full Debug strings are compared directly, never hashed or used as canonical
// product identities. This preserves every field without cross-crate type casts.
pub fn semantic_case(name: &str) -> String {
    let fixture = fixture(name);
    let validation = validate_snapshot(&fixture.profile, &fixture.prior);
    let plan = plan_edits(&fixture.profile, &fixture.prior, &fixture.edits);
    assert_eq!(plan, plan_edits(&fixture.profile, &fixture.prior, &fixture.edits), "repeat determinism {name}");
    let mut output = format!("validation={validation:?};plan={plan:?};");
    match &plan {
        Ok(plan) => {
            assert!(!name.ends_with("denied") && !matches!(name, "insert-existing"|"update-missing"|"delete-missing"|"error-precedence"), "denial unexpectedly succeeded {name}");
            let reference = plan_edits_reference(&fixture.profile, &fixture.prior, &fixture.edits).expect("reference planner");
            assert_eq!(plan.next, reference.next, "canonical bytes, roots and size accounting {name}");
            assert_eq!(plan.staged_blocks, reference.staged_blocks, "staged block selection {name}");
            assert_eq!(plan.reused_block_count, reference.reused_block_count, "reuse selection {name}");
            let diff = diff_maps(&fixture.profile, &fixture.prior, &plan.next.snapshot).expect("real diff");
            assert_eq!(diff, diff_maps(&fixture.profile, &fixture.prior, &plan.next.snapshot).expect("repeat diff"));
            let reference_diff = diff_maps_reference(&fixture.profile, &fixture.prior, &plan.next.snapshot).expect("reference diff");
            assert_eq!(diff.records, reference_diff.records, "exact diff ordering and values {name}");
            assert_eq!(diff.shared_node_count, reference_diff.shared_node_count);
            assert!(diff.complete && !diff.selects_merge_winner);
            write!(output, "diff={diff:?};next-validation={:?};", validate_snapshot(&fixture.profile, &plan.next.snapshot)).unwrap();
        }
        Err(error) => {
            assert!(name.ends_with("denied") || matches!(name, "insert-existing"|"update-missing"|"delete-missing"|"error-precedence"), "valid fixture rejected {name}: {error:?}");
            write!(output, "reference-error={:?};self-diff={:?};", plan_edits_reference(&fixture.profile, &fixture.prior, &fixture.edits), diff_maps(&fixture.profile, &fixture.prior, &fixture.prior)).unwrap();
        }
    }
    output
}
pub fn measurements(variant: &str) {
    for &(name, _, _, _, _, timed) in CASES {
        if !timed { continue; }
        let fixture = fixture(name);
        let next = plan_edits(&fixture.profile, &fixture.prior, &fixture.edits).expect("setup plan").next.snapshot;
        let canonical = edited_entries(&fixture.prior_entries, &fixture.edits).expect("setup entries");
        // Warm identical real operations once outside measurement in each process.
        drop(plan_edits(&fixture.profile, &fixture.prior, &fixture.edits).expect("warm plan"));
        drop(diff_maps(&fixture.profile, &fixture.prior, &next).expect("warm diff"));
        super::measure(variant, name, "validate_prior", || validate_snapshot(&fixture.profile, &fixture.prior).expect("valid prior"));
        super::measure(variant, name, "decode_next_blocks", || next.blocks.iter().map(|block| decode_block(&fixture.profile, block).expect("real block decode")).collect::<Vec<_>>());
        super::measure(variant, name, "index_edit_convert", || edited_entries(&fixture.prior_entries, &fixture.edits).expect("index edits"));
        super::measure(variant, name, "snapshot_clone", || fixture.prior.clone());
        super::measure(variant, name, "entry_clone", || fixture.prior_entries.clone());
        super::measure(variant, name, "sorted_entry_iteration", || canonical.iter().fold(0_usize, |total, entry| total.wrapping_add(std::hint::black_box(entry).key.len()).wrapping_add(entry.value.len())));
        super::measure(variant, name, "canonical_full_build", || build_map(&fixture.profile, &canonical).expect("canonical build"));
        super::measure(variant, name, "plan_edits", || plan_edits(&fixture.profile, &fixture.prior, &fixture.edits).expect("plan"));
        super::measure(variant, name, "diff_maps", || diff_maps(&fixture.profile, &fixture.prior, &next).expect("diff"));
        super::measure(variant, name, "pipeline", || {
            let plan = plan_edits(&fixture.profile, &fixture.prior, &fixture.edits).expect("pipeline plan");
            let diff = diff_maps(&fixture.profile, &fixture.prior, &plan.next.snapshot).expect("pipeline diff");
            (plan, diff)
        });
    }
}
