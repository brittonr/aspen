use super::*;

// r[verify molten.prolly_map.history_independence]
#[test]
fn equal_state_histories_and_compaction_produce_one_root() {
    let profile = profile();
    let empty = build_map(&profile, &[]).expect("empty map");
    let forward_edits = entries().into_iter().map(MapEdit::Insert).collect::<Vec<_>>();
    let mut reverse_edits = forward_edits.clone();
    reverse_edits.reverse();
    let forward = plan_edits(&profile, &empty.snapshot, &forward_edits).expect("forward history");
    let reverse = plan_edits(&profile, &empty.snapshot, &reverse_edits).expect("reverse history");
    assert_eq!(forward.next.snapshot.root.root_ref, reverse.next.snapshot.root.root_ref);
    assert_eq!(forward.next.snapshot.blocks, reverse.next.snapshot.blocks);

    let read = validate_snapshot(&profile, &forward.next.snapshot).expect("read built map");
    let compacted = build_map(&profile, &read.entries).expect("canonical compaction");
    assert_eq!(forward.next.snapshot.root.root_ref, compacted.snapshot.root.root_ref);

    let original = entry(0);
    let changed = SemanticEntry {
        key: original.key.clone(),
        value: vec![UPDATED_VALUE_BYTE; VALUE_BYTES],
    };
    let restored = plan_edits(&profile, &forward.next.snapshot, &[MapEdit::Update(changed), MapEdit::Update(original)])
        .expect("update and restore");
    assert_eq!(restored.next.snapshot.root.root_ref, forward.next.snapshot.root.root_ref);
}

// r[verify molten.prolly_map.history_independence]
#[test]
fn bounded_permutation_property_preserves_the_equal_state_root() {
    let profile = profile();
    let empty = build_map(&profile, &[]).expect("empty map");
    let selected = entries().into_iter().take(PROPERTY_ENTRY_COUNT).collect::<Vec<_>>();
    let expected = build_map(&profile, &selected).expect("expected map");
    let mut orders = Vec::new();
    let mut working = selected;
    collect_permutations(&mut working, 0, &mut orders);
    for order in orders {
        let edits = order.into_iter().map(MapEdit::Insert).collect::<Vec<_>>();
        let plan = plan_edits(&profile, &empty.snapshot, &edits).expect("permutation plan");
        assert_eq!(plan.next.snapshot.root.root_ref, expected.snapshot.root.root_ref);
    }
}

// r[verify molten.prolly_map.boundaries]
#[test]
fn chosen_key_pressure_never_exceeds_the_forced_bound() {
    let profile = profile();
    let build = build_map(&profile, &entries()).expect("bounded build");
    assert!(build.snapshot.blocks.len() > 1);
    assert!(
        build
            .snapshot
            .blocks
            .iter()
            .all(|block| u32::try_from(block.bytes.len()).is_ok_and(|size| size <= profile.max_node_bytes))
    );

    for item in entries() {
        let encoded_size = profile.max_node_bytes;
        assert!(boundary_decision(&profile, &item.key, encoded_size).expect("forced boundary"));
    }
}

// r[verify molten.prolly_map.canonical_nodes]
#[test]
fn malformed_duplicate_overlap_missing_and_tampered_nodes_fail() {
    let profile = profile();
    let duplicate = vec![entry(0), entry(0)];
    assert!(encode_leaf(&profile, &duplicate).expect_err("duplicate key").contains(&ProllyIssue::DuplicateKey));

    let child = build().snapshot.blocks.first().cloned().expect("block");
    let overlap = vec![
        ChildRange {
            min_key: b"a".to_vec(),
            max_key: b"m".to_vec(),
            node_ref: child.node_ref.clone(),
            encoded_len: u32::try_from(child.bytes.len()).expect("block length"),
        },
        ChildRange {
            min_key: b"m".to_vec(),
            max_key: b"z".to_vec(),
            node_ref: child.node_ref,
            encoded_len: u32::try_from(child.bytes.len()).expect("block length"),
        },
    ];
    assert!(encode_internal(&profile, &overlap).expect_err("overlap").contains(&ProllyIssue::ChildRangeOverlap));

    let mut missing = build();
    let top = missing.snapshot.root.top_node_ref.clone();
    let remove_index = missing.snapshot.blocks.iter().position(|block| block.node_ref != top).expect("non-root block");
    let removed = missing.snapshot.blocks.remove(remove_index);
    assert!(
        validate_snapshot(&profile, &missing.snapshot)
            .expect_err("missing child")
            .contains(&ProllyIssue::MissingBlock(removed.node_ref.as_str().to_string()))
    );

    let mut extended = build();
    let extra = build_map(&profile, &[])
        .expect("empty map")
        .snapshot
        .blocks
        .into_iter()
        .next()
        .expect("empty block");
    let extra_ref = extra.node_ref.as_str().to_string();
    extended.snapshot.blocks.push(extra);
    assert!(
        validate_snapshot(&profile, &extended.snapshot)
            .expect_err("extra block")
            .contains(&ProllyIssue::UnexpectedBlock(extra_ref))
    );

    let mut tampered = build();
    let top_block = tampered
        .snapshot
        .blocks
        .iter_mut()
        .find(|block| block.node_ref == tampered.snapshot.root.top_node_ref)
        .expect("top block");
    let final_byte = top_block.bytes.last_mut().expect("encoded byte");
    *final_byte ^= 1;
    let issues = validate_snapshot(&profile, &tampered.snapshot).expect_err("tamper");
    assert!(
        issues.contains(&ProllyIssue::NodeIdentityMismatch)
            || issues.iter().any(|issue| matches!(issue, ProllyIssue::NodeEncodingMalformed(_)))
    );
}

fn collect_permutations(items: &mut [SemanticEntry], start: usize, output: &mut Vec<Vec<SemanticEntry>>) {
    assert!(items.len() <= PROPERTY_ENTRY_COUNT);
    if start == items.len() {
        output.push(items.to_vec());
        return;
    }
    for index in start..items.len() {
        items.swap(start, index);
        collect_permutations(items, start + 1, output);
        items.swap(start, index);
    }
}

#[test]
fn point_range_wrong_profile_and_oversized_values_are_denied() {
    let profile = profile();
    let build = build();
    assert_eq!(
        point_read(&profile, &build.snapshot, &entry(POINT_INDEX).key).expect("point"),
        Some(entry(POINT_INDEX))
    );
    let range =
        range_read(&profile, &build.snapshot, &entry(POINT_INDEX).key, &entry(RANGE_END_INDEX).key).expect("range");
    assert_eq!(range.entries.len(), RANGE_EXPECTED_COUNT);

    let mut wrong = profile.clone();
    wrong.boundary_seed_ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    wrong.profile_ref = derive_profile_ref(&wrong).expect("wrong profile identity");
    assert!(
        validate_snapshot(&wrong, &build.snapshot)
            .expect_err("wrong profile")
            .contains(&ProllyIssue::RootProfileMismatch)
    );

    let mut oversized = entry(0);
    oversized.value =
        vec![ORIGINAL_VALUE_BYTE; usize::try_from(profile.limits.max_value_bytes).expect("value bound") + 1];
    assert!(
        build_map(&profile, &[oversized])
            .expect_err("oversized value")
            .contains(&ProllyIssue::ValueLimitExceeded)
    );
}
