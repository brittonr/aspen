use super::*;

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn canonical_values_encode_to_known_preserves_bytes() {
    for (value, expected) in [
        (CanonicalValue::Integer(0), vec![0xb0, 0x00]),
        (CanonicalValue::Integer(1), vec![0xb0, 0x01, 0x01]),
        (CanonicalValue::Integer(127), vec![0xb0, 0x01, 0x7f]),
        (CanonicalValue::Integer(128), vec![0xb0, 0x02, 0x00, 0x80]),
        (CanonicalValue::Integer(-1), vec![0xb0, 0x01, 0xff]),
        (CanonicalValue::Integer(-129), vec![0xb0, 0x02, 0xff, 0x7f]),
        (CanonicalValue::string("a"), vec![0xb1, 0x01, b'a']),
        (CanonicalValue::Bytes(vec![1, 2]), vec![0xb2, 0x02, 1, 2]),
        (CanonicalValue::symbol("s"), vec![0xb3, 0x01, b's']),
        (CanonicalValue::Sequence(Vec::new()), vec![0xb5, 0x84]),
        (CanonicalValue::record("r", vec![CanonicalValue::Integer(1)]), vec![
            0xb4, 0xb3, 0x01, b'r', 0xb0, 0x01, 0x01, 0x84,
        ]),
    ] {
        assert_eq!(encode_canonical(&value), expected, "{value:?}");
    }
    let long = encode_canonical(&CanonicalValue::string(&"x".repeat(200)));
    assert_eq!(&long[..3], &[0xb1, 0xc8, 0x01], "a 200-byte length is a two-byte varint");
}

#[test]
fn the_projection_record_has_exactly_the_published_fields() {
    let CanonicalValue::Record { label, fields } = projection_value(&projection("node-a", 0, 0, &["a"])) else {
        panic!("a projection is a record");
    };
    assert_eq!(label, PROJECTION_LABEL);
    assert_eq!(fields.len(), PROJECTION_FIELDS.len());
}

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn identical_projections_repeat_their_canonical_ref_and_any_field_changes_it() {
    let first = projection("node-a", 0, 0, &["a"]);
    assert_eq!(projection_ref(&first), projection_ref(&first.clone()));
    let mut moved = first.clone();
    moved.logical_position += 1;
    assert_ne!(projection_ref(&first), projection_ref(&moved));
    let mut reported = first.clone();
    reported.reported_invariants.clear();
    assert_ne!(projection_ref(&first), projection_ref(&reported), "the report is bound, not evaluated");
}

// r[verify molten.fabric_simulation.protocol_projection]
// r[verify molten.fabric_simulation.protocol_oracle_validation]
#[test]
fn admission_refuses_debug_identity_unknown_participants_malformed_refs_and_unbounded_state() {
    let profile = profile();
    assert!(admit_projection(&profile, &projection("node-a", 0, 0, &["a"])).is_ok());

    let mut debug = projection("node-a", 0, 0, &["a"]);
    debug.state_ref = format!("blake3:{}", blake3::hash(format!("{:?}", debug.state).as_bytes()).to_hex());
    assert_eq!(admit_projection(&profile, &debug), Err(ProjectionIssue::NonCanonicalStateIdentity));

    let unknown = projection("node-z", 0, 0, &["a"]);
    assert_eq!(admit_projection(&profile, &unknown), Err(ProjectionIssue::UnknownParticipant("node-z".to_owned())));

    let mut malformed = projection("node-a", 0, 0, &["a"]);
    malformed.history_ref = "blake3:NOT-HEX".to_owned();
    assert_eq!(admit_projection(&profile, &malformed), Err(ProjectionIssue::MalformedRef("history")));

    let mut bounded = profile.clone();
    bounded.bounds.max_state_bytes = 8;
    assert_eq!(
        admit_projection(&bounded, &projection("node-a", 0, 0, &["a"])),
        Err(ProjectionIssue::UnboundedPayload)
    );
}

// r[verify molten.fabric_simulation.protocol_novelty]
#[test]
fn novelty_identity_repeats_for_equal_selected_fields_and_changes_with_them() {
    let fields = [
        NoveltyField::Service,
        NoveltyField::LogicalPosition,
        NoveltyField::State,
    ];
    let first = projection("node-a", 0, 0, &["a"]);
    let mut elsewhere = projection("node-b", 3, 9, &["a"]);
    elsewhere.history_ref = content("unrelated");
    assert_eq!(novelty_identity(&fields, &first), novelty_identity(&fields, &elsewhere));
    let reordered = [
        NoveltyField::State,
        NoveltyField::Service,
        NoveltyField::LogicalPosition,
    ];
    assert_eq!(novelty_identity(&fields, &first), novelty_identity(&reordered, &first));
    let changed = projection("node-a", 1, 1, &["a", "b"]);
    assert_ne!(novelty_identity(&fields, &first), novelty_identity(&fields, &changed));
    assert!(is_content_ref(&novelty_identity(&fields, &first)));
}

// r[verify molten.fabric_simulation.protocol_cost]
#[test]
fn counters_are_monotonic_reset_only_by_schema_and_compared_only_within_one_cohort() {
    let schema = profile().counters;
    let observation = |name: &str, generation, value| CounterObservation {
        name: name.to_owned(),
        generation,
        value,
    };
    let totals = admit_counters(&schema, &[
        observation("transitions", 1, 2),
        observation("transitions", 1, 5),
        observation("repairs", 1, 3),
        observation("repairs", 2, 0),
    ])
    .expect("monotonic with an allowed reset");
    assert_eq!(totals.get("transitions"), Some(&5));
    assert_eq!(totals.get("repairs"), Some(&0));
    let regressed = admit_counters(&schema, &[observation("transitions", 1, 5), observation("transitions", 2, 1)]);
    assert!(matches!(regressed, Err(issues) if matches!(issues[0], CounterIssue::Regression { .. })));
    assert!(admit_counters(&schema, &[observation("wall-clock", 1, 1)]).is_err());

    let cohort = CounterCohort {
        world_ref: content("world"),
        runtime_ref: content("runtime"),
        scheduler_ref: content("scheduler"),
        workload_ref: content("workload"),
        fault_ref: content("faults"),
        counter_schema_ref: content("schema"),
        bounds_ref: content("bounds"),
    };
    let mut fewer = totals.clone();
    fewer.insert("transitions".to_owned(), 3);
    let deltas = compare_counters((&cohort, &totals), (&cohort, &fewer)).expect("one cohort");
    assert_eq!(deltas.iter().find(|delta| delta.name == "transitions").map(|delta| delta.delta), Some(-2));
    let mut changed = cohort.clone();
    changed.scheduler_ref = content("other-schedule");
    assert_eq!(
        compare_counters((&cohort, &totals), (&changed, &fewer)),
        Err(CounterComparisonIssue::CohortMismatch("scheduler"))
    );
}
