//! Infrastructure-free register admission, generation and public projection regressions.

// r[verify molten.consensus.live_register_profile]
#[test]
fn rejects_cross_key_unbounded_and_incompatible_profiles() {
    let mut input = super::support::register();
    input.keys.push("second".into());
    assert_eq!(crate::profile::admit(&input, super::support::content_ref), Err(crate::profile::Issue::SingleKey));
    let mut input = super::support::register();
    input.evidence.max_events = 1;
    assert_eq!(crate::profile::admit(&input, super::support::content_ref), Err(crate::profile::Issue::Bounds));
    let mut input = super::support::register();
    input.initial_value = Some(7);
    assert_eq!(crate::profile::admit(&input, super::support::content_ref), Err(crate::profile::Issue::InitialValue));
    let mut input = super::support::register();
    input.concurrency = 3;
    assert!(crate::profile::admit(&input, super::support::content_ref).is_err());
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn canonical_reference_port_runs_only_for_structurally_admitted_profiles() {
    let mut calls = 0_u32;
    let mut invalid = super::support::register();
    invalid.consistency_model = "linearizable-queue".into();
    let denied = crate::profile::admit(&invalid, |_| {
        calls += 1;
        Ok(alloc::string::String::new())
    });
    assert_eq!(denied, Err(crate::profile::Issue::Model));
    assert_eq!(calls, 0, "a denied profile never reaches the identity port");

    assert_eq!(
        crate::profile::admit(&super::support::register(), |_| Err(crate::profile::Issue::Encoding)),
        Err(crate::profile::Issue::Encoding)
    );
    let bound = alloc::format!("blake3:{}", "c".repeat(64));
    let admitted = crate::profile::admit(&super::support::register(), |_| Ok(bound.clone())).expect("admitted");
    assert_eq!(admitted.profile_ref(), bound);
    assert_eq!(admitted.profile(), &super::support::register());
}

// r[verify molten.consensus.live_operation_generator]
#[test]
fn seeded_plan_preserves_scope_and_requires_every_final_endpoint() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    assert_eq!(plan, crate::generator::generate(&input).expect("admitted plan"));
    let final_operations = &plan.operations[10..];
    for (operation, endpoint) in final_operations.iter().zip(&input.profile().endpoints) {
        assert_eq!(&operation.endpoint, endpoint);
        assert_eq!(operation.function, crate::generator::Function::Read);
        assert_eq!(operation.object_key, "k");
    }
    let mut changed = plan.clone();
    changed.operations[0].id.push('x');
    assert!(crate::generator::validate_plan(&input, &changed).is_err());
    let mut changed = plan.clone();
    changed.choices[0].value_draw ^= 1;
    assert!(crate::generator::validate_plan(&input, &changed).is_err());
    let encoded = serde_json::to_value(&plan).expect("serialize");
    assert_eq!(encoded["operations"][11]["value"], serde_json::json!({"type":"null"}));
}

// r[verify molten.consensus.live_operation_history]
#[test]
fn committed_lost_response_is_uncertain_and_retry_identity_is_stable() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let operation = &plan.operations[0];
    let first = crate::history::project_attempt(
        &input,
        operation,
        &super::support::observation(crate::history::PublicOutcome::LostResponse),
    )
    .expect("uncertain");
    assert!(matches!(first[1], crate::history::Event::Complete {
        outcome: crate::history::Outcome::Info,
        ..
    }));
    let mut retry = super::support::observation(crate::history::PublicOutcome::Timeout);
    retry.ordinal = 1;
    let retried = crate::history::project_attempt(&input, operation, &retry).expect("retry");
    match &retried[0] {
        crate::history::Event::Invoke {
            logical_operation_id,
            attempt_id,
            retry_of_attempt,
            ..
        } => {
            assert_eq!(logical_operation_id, &operation.id);
            assert_ne!(attempt_id, retry_of_attempt.as_ref().expect("prior"));
            assert_eq!(retry_of_attempt.as_deref(), Some(crate::history::attempt_id(operation, 0).as_str()));
        }
        _ => panic!("invoke required"),
    }
}

#[test]
fn public_read_outputs_reject_missing_and_out_of_corpus_values() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let read = plan.operations.last().expect("final read");
    for output in [None, Some(crate::generator::Value::Integer { value: 999 })] {
        assert!(
            crate::history::project_attempt(
                &input,
                read,
                &super::support::observation(crate::history::PublicOutcome::Acknowledged(output))
            )
            .is_err()
        );
    }
    for output in [crate::generator::Value::Null, crate::generator::Value::Integer {
        value: 7,
    }] {
        let events = crate::history::project_attempt(
            &input,
            read,
            &super::support::observation(crate::history::PublicOutcome::Acknowledged(Some(output))),
        )
        .expect("read");
        assert!(matches!(events[1], crate::history::Event::Complete {
            outcome: crate::history::Outcome::Ok,
            ..
        }));
    }
    let events = crate::history::project_attempt(
        &input,
        read,
        &super::support::observation(crate::history::PublicOutcome::MalformedResponse),
    )
    .expect("uncertain malformed response");
    assert!(matches!(events[1], crate::history::Event::Complete {
        outcome: crate::history::Outcome::Info,
        ..
    }));
}

#[test]
fn history_rejects_changed_ids_duplicate_completions_and_incomplete_pairs() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let complete = super::support::history(&input);
    assert_eq!(super::support::validate(&input, &plan, &complete), Ok(()));
    let mut changed = complete.clone();
    if let crate::history::Event::Invoke {
        logical_operation_id, ..
    } = &mut changed.events[0]
    {
        logical_operation_id.push('x');
    }
    assert_eq!(super::support::validate(&input, &plan, &changed), Err(crate::history::Issue::Identity));
    let mut changed = complete.clone();
    changed.events.pop();
    assert_eq!(super::support::validate(&input, &plan, &changed), Err(crate::history::Issue::Incomplete));
    let mut changed = complete;
    changed.events[3] = changed.events[1].clone();
    assert!(super::support::validate(&input, &plan, &changed).is_err());
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn encoded_size_port_enforces_the_evidence_byte_bound_after_the_header() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let complete = super::support::history(&input);
    let max_bytes = input.profile().evidence.max_bytes;
    assert_eq!(crate::history::validate(&input, &plan, &complete, |_| Ok(max_bytes)), Ok(()));
    assert_eq!(
        crate::history::validate(&input, &plan, &complete, |_| Ok(max_bytes + 1)),
        Err(crate::history::Issue::Bounds)
    );
    assert_eq!(
        crate::history::validate(&input, &plan, &complete, |_| Err(crate::history::Issue::Bounds)),
        Err(crate::history::Issue::Bounds)
    );
    let mut stale_header = complete;
    stale_header.schema_version = 1;
    let mut is_measured = false;
    assert_eq!(
        crate::history::validate(&input, &plan, &stale_header, |_| {
            is_measured = true;
            Ok(0)
        }),
        Err(crate::history::Issue::Profile)
    );
    assert!(!is_measured, "a stale header is denied before the document is measured");
}

#[test]
fn write_acknowledgment_does_not_invent_a_read_output() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let write = plan
        .operations
        .iter()
        .find(|operation| operation.function == crate::generator::Function::Write)
        .expect("write corpus");
    let events = crate::history::project_attempt(
        &input,
        write,
        &super::support::observation(crate::history::PublicOutcome::Acknowledged(None)),
    )
    .expect("acknowledged write");
    assert!(matches!(events[1], crate::history::Event::Complete {
        outcome: crate::history::Outcome::Ok,
        output: None,
        ..
    }));
    assert!(
        crate::history::project_attempt(
            &input,
            write,
            &super::support::observation(crate::history::PublicOutcome::Acknowledged(Some(write.value)))
        )
        .is_err()
    );
    let denial = crate::history::project_attempt(
        &input,
        write,
        &super::support::observation(crate::history::PublicOutcome::PreEffectDenied),
    )
    .expect("pre-effect denial");
    assert!(matches!(denial[1], crate::history::Event::Complete {
        outcome: crate::history::Outcome::Fail,
        ..
    }));
}

#[test]
fn acknowledged_operation_cannot_be_retried_and_pending_has_no_completion() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let mut complete = super::support::history(&input);
    let mut retry = super::support::observation(crate::history::PublicOutcome::Timeout);
    retry.ordinal = 1;
    retry.event_index = complete.events.len() as u64;
    retry.started_ns = 1000;
    retry.completed_ns = 1001;
    complete
        .events
        .extend(crate::history::project_attempt(&input, &plan.operations[0], &retry).expect("pair"));
    complete.completeness.invocations += 1;
    complete.completeness.completions += 1;
    assert_eq!(super::support::validate(&input, &plan, &complete), Err(crate::history::Issue::Retry));
    let pending = crate::history::project_attempt(
        &input,
        &plan.operations[0],
        &super::support::observation(crate::history::PublicOutcome::Pending),
    )
    .expect("pending invoke");
    assert!(matches!(pending.as_slice(), [crate::history::Event::Invoke { .. }]));
}

#[test]
fn selected_fault_cannot_be_counted_as_observed_effect() {
    let input = super::support::admitted();
    let plan = crate::generator::generate(&input).expect("admitted plan");
    let mut complete = super::support::history(&input);
    let index = complete.events.len() as u64;
    complete.events.push(crate::history::Event::Fault {
        event_index: index,
        fault_attempt_id: "restart-a".into(),
        phase: "selected".into(),
        effect_record_ref: None,
        controller_time_ns: 1000,
    });
    complete.completeness.observed_faults = 1;
    assert_eq!(super::support::validate(&input, &plan, &complete), Err(crate::history::Issue::Incomplete));
    complete.completeness.observed_faults = 0;
    assert_eq!(super::support::validate(&input, &plan, &complete), Ok(()));
    complete.events.push(crate::history::Event::Fault {
        event_index: index + 1,
        fault_attempt_id: "restart-a".into(),
        phase: "observed".into(),
        effect_record_ref: Some(super::support::observation(crate::history::PublicOutcome::Pending).source_artifact),
        controller_time_ns: 1001,
    });
    assert_eq!(super::support::validate(&input, &plan, &complete), Err(crate::history::Issue::Order));
}
