//! Infrastructure-free register admission, generation and public projection regressions.

#[path = "corefixtures.rs"]
mod support;

// r[verify molten.consensus.live_register_profile]
#[test]
fn rejects_cross_key_unbounded_and_incompatible_profiles() {
    let mut input = support::register();
    input.keys.push("second".into());
    assert_eq!(
        crate::live_reliability::profile::admit(&input),
        Err(crate::live_reliability::profile::Issue::SingleKey)
    );
    let mut input = support::register();
    input.evidence.max_events = 1;
    assert_eq!(
        crate::live_reliability::profile::admit(&input),
        Err(crate::live_reliability::profile::Issue::Bounds)
    );
    let mut input = support::register();
    input.initial_value = Some(7);
    assert_eq!(
        crate::live_reliability::profile::admit(&input),
        Err(crate::live_reliability::profile::Issue::InitialValue)
    );
    let mut input = support::register();
    input.concurrency = 3;
    assert!(crate::live_reliability::profile::admit(&input).is_err());
}

// r[verify molten.consensus.live_operation_generator]
#[test]
fn seeded_plan_preserves_scope_and_requires_every_final_endpoint() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    assert_eq!(plan, crate::live_reliability::generator::generate(&input).expect("admitted plan"));
    let final_operations = &plan.operations[10..];
    for (operation, endpoint) in final_operations.iter().zip(&input.profile().endpoints) {
        assert_eq!(&operation.endpoint, endpoint);
        assert_eq!(operation.function, crate::live_reliability::generator::Function::Read);
        assert_eq!(operation.object_key, "k");
    }
    let mut changed = plan.clone();
    changed.operations[0].id.push('x');
    assert!(crate::live_reliability::generator::validate_plan(&input, &changed).is_err());
    let mut changed = plan.clone();
    changed.choices[0].value_draw ^= 1;
    assert!(crate::live_reliability::generator::validate_plan(&input, &changed).is_err());
    let encoded = serde_json::to_value(&plan).expect("serialize");
    assert_eq!(encoded["operations"][11]["value"], serde_json::json!({"type":"null"}));
}

// r[verify molten.consensus.live_operation_history]
#[test]
fn committed_lost_response_is_uncertain_and_retry_identity_is_stable() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let operation = &plan.operations[0];
    let first = crate::live_reliability::history::project_attempt(
        &input,
        operation,
        &support::observation(crate::live_reliability::history::PublicOutcome::LostResponse),
    )
    .expect("uncertain");
    assert!(matches!(first[1], crate::live_reliability::history::Event::Complete {
        outcome: crate::live_reliability::history::Outcome::Info,
        ..
    }));
    let mut retry = support::observation(crate::live_reliability::history::PublicOutcome::Timeout);
    retry.ordinal = 1;
    let retried = crate::live_reliability::history::project_attempt(&input, operation, &retry).expect("retry");
    match &retried[0] {
        crate::live_reliability::history::Event::Invoke {
            logical_operation_id,
            attempt_id,
            retry_of_attempt,
            ..
        } => {
            assert_eq!(logical_operation_id, &operation.id);
            assert_ne!(attempt_id, retry_of_attempt.as_ref().expect("prior"));
            assert_eq!(
                retry_of_attempt.as_deref(),
                Some(crate::live_reliability::history::attempt_id(operation, 0).as_str())
            );
        }
        _ => panic!("invoke required"),
    }
}

#[test]
fn public_read_outputs_reject_missing_and_out_of_corpus_values() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let read = plan.operations.last().expect("final read");
    for output in [
        None,
        Some(crate::live_reliability::generator::Value::Integer { value: 999 }),
    ] {
        assert!(
            crate::live_reliability::history::project_attempt(
                &input,
                read,
                &support::observation(crate::live_reliability::history::PublicOutcome::Acknowledged(output))
            )
            .is_err()
        );
    }
    for output in [
        crate::live_reliability::generator::Value::Null,
        crate::live_reliability::generator::Value::Integer { value: 7 },
    ] {
        let events = crate::live_reliability::history::project_attempt(
            &input,
            read,
            &support::observation(crate::live_reliability::history::PublicOutcome::Acknowledged(Some(output))),
        )
        .expect("read");
        assert!(matches!(events[1], crate::live_reliability::history::Event::Complete {
            outcome: crate::live_reliability::history::Outcome::Ok,
            ..
        }));
    }
    let events = crate::live_reliability::history::project_attempt(
        &input,
        read,
        &support::observation(crate::live_reliability::history::PublicOutcome::MalformedResponse),
    )
    .expect("uncertain malformed response");
    assert!(matches!(events[1], crate::live_reliability::history::Event::Complete {
        outcome: crate::live_reliability::history::Outcome::Info,
        ..
    }));
}

#[test]
fn history_rejects_changed_ids_duplicate_completions_and_incomplete_pairs() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let complete = support::history(&input);
    assert_eq!(crate::live_reliability::history::validate(&input, &plan, &complete), Ok(()));
    let mut changed = complete.clone();
    if let crate::live_reliability::history::Event::Invoke {
        logical_operation_id, ..
    } = &mut changed.events[0]
    {
        logical_operation_id.push('x');
    }
    assert_eq!(
        crate::live_reliability::history::validate(&input, &plan, &changed),
        Err(crate::live_reliability::history::Issue::Identity)
    );
    let mut changed = complete.clone();
    changed.events.pop();
    assert_eq!(
        crate::live_reliability::history::validate(&input, &plan, &changed),
        Err(crate::live_reliability::history::Issue::Incomplete)
    );
    let mut changed = complete;
    changed.events[3] = changed.events[1].clone();
    assert!(crate::live_reliability::history::validate(&input, &plan, &changed).is_err());
}

#[test]
fn write_acknowledgment_does_not_invent_a_read_output() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let write = plan
        .operations
        .iter()
        .find(|operation| operation.function == crate::live_reliability::generator::Function::Write)
        .expect("write corpus");
    let events = crate::live_reliability::history::project_attempt(
        &input,
        write,
        &support::observation(crate::live_reliability::history::PublicOutcome::Acknowledged(None)),
    )
    .expect("acknowledged write");
    assert!(matches!(events[1], crate::live_reliability::history::Event::Complete {
        outcome: crate::live_reliability::history::Outcome::Ok,
        output: None,
        ..
    }));
    assert!(
        crate::live_reliability::history::project_attempt(
            &input,
            write,
            &support::observation(crate::live_reliability::history::PublicOutcome::Acknowledged(Some(write.value)))
        )
        .is_err()
    );
    let denial = crate::live_reliability::history::project_attempt(
        &input,
        write,
        &support::observation(crate::live_reliability::history::PublicOutcome::PreEffectDenied),
    )
    .expect("pre-effect denial");
    assert!(matches!(denial[1], crate::live_reliability::history::Event::Complete {
        outcome: crate::live_reliability::history::Outcome::Fail,
        ..
    }));
}

#[test]
fn acknowledged_operation_cannot_be_retried_and_pending_has_no_completion() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let mut complete = support::history(&input);
    let mut retry = support::observation(crate::live_reliability::history::PublicOutcome::Timeout);
    retry.ordinal = 1;
    retry.event_index = complete.events.len() as u64;
    retry.started_ns = 1000;
    retry.completed_ns = 1001;
    complete
        .events
        .extend(crate::live_reliability::history::project_attempt(&input, &plan.operations[0], &retry).expect("pair"));
    complete.completeness.invocations += 1;
    complete.completeness.completions += 1;
    assert_eq!(
        crate::live_reliability::history::validate(&input, &plan, &complete),
        Err(crate::live_reliability::history::Issue::Retry)
    );
    let pending = crate::live_reliability::history::project_attempt(
        &input,
        &plan.operations[0],
        &support::observation(crate::live_reliability::history::PublicOutcome::Pending),
    )
    .expect("pending invoke");
    assert!(matches!(pending.as_slice(), [crate::live_reliability::history::Event::Invoke { .. }]));
}

#[test]
fn selected_fault_cannot_be_counted_as_observed_effect() {
    let input = support::admitted();
    let plan = crate::live_reliability::generator::generate(&input).expect("admitted plan");
    let mut complete = support::history(&input);
    let index = complete.events.len() as u64;
    complete.events.push(crate::live_reliability::history::Event::Fault {
        event_index: index,
        fault_attempt_id: "restart-a".into(),
        phase: "selected".into(),
        effect_record_ref: None,
        controller_time_ns: 1000,
    });
    complete.completeness.observed_faults = 1;
    assert_eq!(
        crate::live_reliability::history::validate(&input, &plan, &complete),
        Err(crate::live_reliability::history::Issue::Incomplete)
    );
    complete.completeness.observed_faults = 0;
    assert_eq!(crate::live_reliability::history::validate(&input, &plan, &complete), Ok(()));
    complete.events.push(crate::live_reliability::history::Event::Fault {
        event_index: index + 1,
        fault_attempt_id: "restart-a".into(),
        phase: "observed".into(),
        effect_record_ref: Some(
            support::observation(crate::live_reliability::history::PublicOutcome::Pending).source_artifact,
        ),
        controller_time_ns: 1001,
    });
    assert_eq!(
        crate::live_reliability::history::validate(&input, &plan, &complete),
        Err(crate::live_reliability::history::Issue::Order)
    );
}
