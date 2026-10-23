use super::*;

fn frame(label: &str, payload: &[u8]) -> Vec<u8> {
    encode_block_frame(&digest(label), payload, MAX_OBJECT).expect("frame")
}

// r[verify aspen.marble_store.identity]
#[test]
fn recovery_rebuilds_the_index_and_catalog_from_recovered_objects_only() {
    let catalog = MarbleCatalog {
        heads: std::collections::BTreeMap::new(),
        pins: std::collections::BTreeSet::from([digest("pinned")]),
        retention_generation: 3,
    };
    let mut recovery = IndexRecovery::new(layout(16), MAX_OBJECT);
    recovery.observe(0, &encode_catalog(&catalog).expect("catalog"));
    recovery.observe(1_025, &frame("b", b"b"));
    recovery.observe(1_024, &frame("a", b"a"));
    let recovered = recovery.finish().expect("recovered");
    let mut expected = vec![(digest("a"), 1_024), (digest("b"), 1_025)];
    expected.sort();
    assert_eq!(recovered.entries, expected);
    assert_eq!(recovered.catalog, catalog);
    assert!(recovered.redundant_object_ids.is_empty());

    let fresh = IndexRecovery::new(layout(16), MAX_OBJECT).finish().expect("fresh store");
    assert!(fresh.entries.is_empty(), "an unrecovered batch contributes no mapping");
    assert_eq!(fresh.catalog, MarbleCatalog::default());
}

// r[verify aspen.marble_store.identity]
#[test]
fn a_repeated_digest_keeps_the_lowest_object_and_a_conflicting_payload_refuses_recovery() {
    let mut recovery = IndexRecovery::new(layout(16), MAX_OBJECT);
    recovery.observe(1_030, &frame("a", b"a"));
    recovery.observe(1_026, &frame("a", b"a"));
    let recovered = recovery.finish().expect("identical payloads");
    assert_eq!(recovered.entries, vec![(digest("a"), 1_026)]);
    assert_eq!(recovered.redundant_object_ids, vec![1_030]);

    let mut conflicting = IndexRecovery::new(layout(16), MAX_OBJECT);
    conflicting.observe(1_024, &frame("a", b"a"));
    conflicting.observe(1_025, &frame("a", b"other"));
    assert_eq!(conflicting.finish(), Err(vec![MarbleStoreIssue::RecoveredDigestConflict]));
}

#[test]
fn foreign_identifiers_corrupt_frames_and_corrupt_catalogs_refuse_recovery() {
    let mut recovery = IndexRecovery::new(layout(16), MAX_OBJECT);
    recovery.observe(500, &frame("a", b"a"));
    recovery.observe(1_024, &[9, 1, 2]);
    recovery.observe(0, b"not a catalog");
    let issues = recovery.finish().expect_err("refused");
    assert!(issues.contains(&MarbleStoreIssue::ForeignObjectId(500)));
    assert!(issues.contains(&MarbleStoreIssue::UnknownFrameVersion(9)));
    assert!(issues.iter().any(|issue| matches!(issue, MarbleStoreIssue::InvalidCatalog(_))));
}
