fn run_binding_test_value(previous_run_ref: Option<&str>) -> IoValue {
    node_run_binding_value(&NodeRunBindingValueInput {
        state_root_ref: &test_ref("run-state-root"),
        startup_receipt_ref: &test_ref("run-startup"),
        previous_run_ref,
    })
    .expect("run binding")
}

fn run_shutdown_test_value() -> IoValue {
    node_run_shutdown_observation_value(&NodeRunShutdownObservationValueInput {
        run_ref: &test_ref("completion-run"),
        startup_receipt_ref: &test_ref("completion-startup"),
        shutdown_receipt_ref: &test_ref("completion-shutdown"),
        control_receipt_ref: &test_ref("completion-control"),
    })
    .expect("run shutdown observation")
}

fn run_test_record_fields(value: &IoValue, tag: &str, arity: usize) -> Vec<IoValue> {
    let fields = value.collect_simple_record(tag, Some(arity)).expect("record fields");
    (0..arity).map(|index| value_to_iovalue(&fields[index])).collect()
}

#[test]
fn run_binding_distinguishes_identical_startups_by_previous_run_chain() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let first_value = run_binding_test_value(None);
    let first = parse_node_run_binding(&first_value).expect("first run");
    let second_value = run_binding_test_value(Some(&first.run_ref));
    let second = parse_node_run_binding(&second_value).expect("second run");
    let third_value = run_binding_test_value(Some(&second.run_ref));
    let third = parse_node_run_binding(&third_value).expect("third run");

    assert_eq!(first.state_root_ref, test_ref("run-state-root"));
    assert_eq!(first.startup_receipt_ref, second.startup_receipt_ref);
    assert_eq!(second.startup_receipt_ref, third.startup_receipt_ref);
    assert_eq!(first.previous_run_ref, None);
    assert_eq!(second.previous_run_ref.as_deref(), Some(first.run_ref.as_str()));
    assert_eq!(third.previous_run_ref.as_deref(), Some(second.run_ref.as_str()));
    assert_ne!(first.run_ref, second.run_ref);
    assert_ne!(first.run_ref, third.run_ref);
    assert_ne!(second.run_ref, third.run_ref);
    assert_eq!(parse_node_run_binding(&run_binding_test_value(None)).expect("repeat first"), first);
    assert_eq!(
        parse_node_run_binding(&run_binding_test_value(Some(&first.run_ref))).expect("repeat second"),
        second,
    );
    assert_eq!(first.run_ref, canonical_hash(&first_value).expect("exact run hash"));
    assert_eq!(first.value, first_value);
    assert_eq!(second.value, second_value);
}

#[test]
fn run_binding_preserves_exact_accepted_optional_reference_value() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let previous = test_ref("previous-run");
    let wrapped_value = run_binding_test_value(Some(&previous));
    let mut fields = run_test_record_fields(&wrapped_value, "node-control-run-v1", 4);
    fields[3] = record("previous-run", vec![string(&previous)]);
    let direct_value = record("node-control-run-v1", fields);
    let wrapped = parse_node_run_binding(&wrapped_value).expect("wrapped previous ref");
    let direct = parse_node_run_binding(&direct_value).expect("established direct optional ref");

    assert_eq!(direct.previous_run_ref.as_deref(), Some(previous.as_str()));
    assert_eq!(direct.run_ref, canonical_hash(&direct_value).expect("exact direct hash"));
    assert_ne!(direct.run_ref, wrapped.run_ref);
    assert_eq!(direct.value, direct_value);
}

#[test]
fn run_shutdown_observation_preserves_exact_completion_crosslinks() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let value = run_shutdown_test_value();
    let observation = parse_node_run_shutdown_observation(&value).expect("completion observation");
    assert_eq!(observation.run_ref, test_ref("completion-run"));
    assert_eq!(observation.startup_receipt_ref, test_ref("completion-startup"));
    assert_eq!(observation.shutdown_receipt_ref, test_ref("completion-shutdown"));
    assert_eq!(observation.control_receipt_ref, test_ref("completion-control"));
    assert_eq!(observation.observation_ref, canonical_hash(&value).expect("exact observation hash"));
    assert_eq!(observation.value, value);
    assert_eq!(
        parse_node_run_shutdown_observation(&run_shutdown_test_value()).expect("repeat observation"),
        observation,
    );
}

#[test]
fn run_binding_rejects_mismatched_record_contracts() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let value = run_binding_test_value(None);
    let fields = run_test_record_fields(&value, "node-control-run-v1", 4);
    assert!(parse_node_run_binding(&record("node-control-run-v2", fields.clone())).is_err());
    assert!(parse_node_run_binding(&record("node-control-run-v1", fields[..3].to_vec())).is_err());
    let mut extra = fields.clone();
    extra.push(record("extra", vec![]));
    assert!(parse_node_run_binding(&record("node-control-run-v1", extra)).is_err());
    let mut wrong_schema = fields.clone();
    wrong_schema[0] = string(crate::preserves_rail::NODE_CONTROL_RUN_SHUTDOWN_SCHEMA);
    assert!(parse_node_run_binding(&record("node-control-run-v1", wrong_schema)).is_err());
    for index in 1..4 {
        let mut wrong_field = fields.clone();
        wrong_field[index] = record("unexpected", vec![string(test_ref("valid-ref"))]);
        assert!(parse_node_run_binding(&record("node-control-run-v1", wrong_field)).is_err());
    }
    let mut swapped = fields.clone();
    swapped.swap(1, 2);
    assert!(parse_node_run_binding(&record("node-control-run-v1", swapped)).is_err());
    let mut wrong_arity = fields;
    wrong_arity[1] = record("state-root", vec![]);
    assert!(parse_node_run_binding(&record("node-control-run-v1", wrong_arity)).is_err());
}

#[test]
fn run_shutdown_observation_rejects_mismatched_record_contracts() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let value = run_shutdown_test_value();
    let fields = run_test_record_fields(&value, "node-control-run-shutdown-v1", 5);
    assert!(parse_node_run_shutdown_observation(&record("node-control-run-v1", fields.clone())).is_err());
    assert!(
        parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", fields[..4].to_vec())).is_err()
    );
    let mut extra = fields.clone();
    extra.push(record("extra", vec![]));
    assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", extra)).is_err());
    let mut wrong_schema = fields.clone();
    wrong_schema[0] = string(crate::preserves_rail::NODE_CONTROL_RUN_SCHEMA);
    assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", wrong_schema)).is_err());
    for index in 1..5 {
        let mut wrong_field = fields.clone();
        wrong_field[index] = record("unexpected", vec![string(test_ref("valid-ref"))]);
        assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", wrong_field)).is_err());
    }
    let mut swapped = fields.clone();
    swapped.swap(1, 2);
    assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", swapped)).is_err());
    let mut wrong_arity = fields;
    wrong_arity[4] = record("control", vec![string(test_ref("one")), string(test_ref("two"))]);
    assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", wrong_arity)).is_err());
}

#[test]
fn run_binding_rejects_invalid_required_and_previous_refs() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let value = run_binding_test_value(None);
    let fields = run_test_record_fields(&value, "node-control-run-v1", 4);
    for invalid in [
        "",
        "blake3:short",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        for (index, label) in [(1, "state-root"), (2, "startup")] {
            let mut malformed = fields.clone();
            malformed[index] = record(label, vec![string(invalid)]);
            assert!(parse_node_run_binding(&record("node-control-run-v1", malformed)).is_err());
        }
        let valid = test_ref("valid-ref");
        for (state_root_ref, startup_receipt_ref, previous_run_ref) in [
            (invalid, valid.as_str(), None),
            (valid.as_str(), invalid, None),
            (valid.as_str(), valid.as_str(), Some(invalid)),
        ] {
            assert!(
                node_run_binding_value(&NodeRunBindingValueInput {
                    state_root_ref,
                    startup_receipt_ref,
                    previous_run_ref,
                })
                .is_err()
            );
        }
        let mut malformed = fields.clone();
        malformed[3] = record("previous-run", vec![record("some", vec![string(invalid)])]);
        assert!(parse_node_run_binding(&record("node-control-run-v1", malformed)).is_err());
    }
    for optional in [
        record("none", vec![string(test_ref("unexpected"))]),
        record("some", vec![]),
        record("some", vec![string(test_ref("one")), string(test_ref("two"))]),
        record("unknown", vec![]),
    ] {
        let mut malformed = fields.clone();
        malformed[3] = record("previous-run", vec![optional]);
        assert!(parse_node_run_binding(&record("node-control-run-v1", malformed)).is_err());
    }
}

#[test]
fn run_shutdown_observation_rejects_invalid_refs_in_every_crosslink() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let value = run_shutdown_test_value();
    let fields = run_test_record_fields(&value, "node-control-run-shutdown-v1", 5);
    let valid = test_ref("valid-ref");
    for index in 0..4 {
        for invalid in [
            "",
            "blake3:short",
            "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let mut refs = [valid.as_str(); 4];
            refs[index] = invalid;
            assert!(
                node_run_shutdown_observation_value(&NodeRunShutdownObservationValueInput {
                    run_ref: refs[0],
                    startup_receipt_ref: refs[1],
                    shutdown_receipt_ref: refs[2],
                    control_receipt_ref: refs[3],
                })
                .is_err()
            );
            let mut malformed = fields.clone();
            let label = ["run", "startup", "shutdown", "control"][index];
            malformed[index + 1] = record(label, vec![string(invalid)]);
            assert!(parse_node_run_shutdown_observation(&record("node-control-run-shutdown-v1", malformed)).is_err());
        }
    }
}
