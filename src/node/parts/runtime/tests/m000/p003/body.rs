struct LifecycleFixture {
    run_ref: String,
    startup_ref: String,
    control: ControlReceipt,
    shutdown: NodeShutdownReceipt,
    observation: NodeRunShutdownObservation,
}

fn lifecycle_shutdown(startup_ref: &str, decision: &str) -> NodeShutdownReceipt {
    let value = node_shutdown_receipt_value(&ShutdownReceiptValueInput {
        decision,
        startup_receipt_ref: startup_ref,
        adapter_receipts: &admission_adapters(),
        drained_job_refs: &[],
        index_receipt_refs: &[test_ref("shutdown-index")],
        diagnostics: &[],
    })
    .expect("shutdown receipt");
    parse_node_shutdown_receipt(&value).expect("parsed shutdown receipt")
}

fn lifecycle_control(startup_ref: &str, decision: &str, subreceipts: &[String]) -> ControlReceipt {
    let request =
        admission_request("shutdown", &[test_ref("authority")], &[test_ref("policy")], &[test_ref("resource")]);
    let value = control_receipt_value(&ControlReceiptValueInput {
        decision,
        request: &request,
        startup_receipt_ref: startup_ref,
        authority_receipt_refs: &[test_ref("authority-receipt")],
        resource_receipt_refs: &[test_ref("resource-receipt")],
        subreceipt_refs: subreceipts,
        diagnostics: &[],
    })
    .expect("control receipt");
    parse_control_receipt(&value).expect("parsed control receipt")
}

fn lifecycle_observation(
    run_ref: &str,
    startup_ref: &str,
    shutdown_ref: &str,
    control_ref: &str,
) -> NodeRunShutdownObservation {
    let value = node_run_shutdown_observation_value(&NodeRunShutdownObservationValueInput {
        run_ref,
        startup_receipt_ref: startup_ref,
        shutdown_receipt_ref: shutdown_ref,
        control_receipt_ref: control_ref,
    })
    .expect("shutdown observation");
    parse_node_run_shutdown_observation(&value).expect("parsed shutdown observation")
}

fn lifecycle_run(startup_ref: &str, previous_run_ref: Option<&str>) -> NodeRunBinding {
    let value = node_run_binding_value(&NodeRunBindingValueInput {
        state_root_ref: &test_ref("lifecycle-state-root"),
        startup_receipt_ref: startup_ref,
        previous_run_ref,
    })
    .expect("run binding");
    parse_node_run_binding(&value).expect("parsed run binding")
}

fn lifecycle_fixture() -> LifecycleFixture {
    let startup_ref = test_ref("lifecycle-startup");
    let run_ref = lifecycle_run(&startup_ref, None).run_ref;
    let shutdown = lifecycle_shutdown(&startup_ref, "pass");
    let control = lifecycle_control(&startup_ref, "pass", std::slice::from_ref(&shutdown.receipt_ref));
    let observation = lifecycle_observation(&run_ref, &startup_ref, &shutdown.receipt_ref, &control.receipt_ref);
    LifecycleFixture {
        run_ref,
        startup_ref,
        control,
        shutdown,
        observation,
    }
}

impl LifecycleFixture {
    fn input(&self) -> NodeRunLifecycleInput<'_> {
        NodeRunLifecycleInput {
            operation: "shutdown",
            is_duplicate: false,
            current_run_ref: &self.run_ref,
            current_startup_receipt_ref: &self.startup_ref,
            active_run_ref: None,
            shutdown_observation: Some(&self.observation),
            shutdown_receipt: Some(&self.shutdown),
            control_receipt: Some(&self.control),
        }
    }
}

#[test]
fn lifecycle_current_completion_stops_independently_of_duplicate_provenance() {
    // r[verify molten.audit_f14.duplicate_scope]
    // r[verify molten.audit_f14.current_binding]
    let fixture = lifecycle_fixture();
    for is_duplicate in [false, true] {
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            is_duplicate,
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Stopped);
        assert_eq!(outcome.is_duplicate, is_duplicate);
    }
}

#[test]
fn lifecycle_historical_duplicate_preserves_active_restart_with_equal_startup() {
    // r[verify molten.audit_f14.duplicate_scope]
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    let previous = lifecycle_fixture();
    let current = lifecycle_run(&previous.startup_ref, Some(&previous.run_ref));
    assert_ne!(current.run_ref, previous.run_ref);
    assert_eq!(current.startup_receipt_ref, previous.startup_ref);
    let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        is_duplicate: true,
        current_run_ref: &current.run_ref,
        active_run_ref: Some(&current.run_ref),
        shutdown_observation: None,
        shutdown_receipt: None,
        ..previous.input()
    });
    assert_eq!(outcome.lifecycle, NodeRunLifecycle::Active);
    assert!(outcome.is_duplicate);

    let stale_completion = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        is_duplicate: true,
        current_run_ref: &current.run_ref,
        ..previous.input()
    });
    assert_eq!(stale_completion.lifecycle, NodeRunLifecycle::Unresolved);
    assert!(stale_completion.is_duplicate);
}

#[test]
fn lifecycle_historical_startup_does_not_override_current_active_lock() {
    // r[verify molten.audit_f14.preserve_state]
    let historical = lifecycle_fixture();
    let startup_ref = test_ref("different-current-startup");
    let current = lifecycle_run(&startup_ref, Some(&historical.run_ref));
    let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        current_run_ref: &current.run_ref,
        current_startup_receipt_ref: &startup_ref,
        active_run_ref: Some(&current.run_ref),
        shutdown_observation: None,
        shutdown_receipt: None,
        is_duplicate: true,
        ..historical.input()
    });
    assert_eq!(outcome.lifecycle, NodeRunLifecycle::Active);
    assert!(outcome.is_duplicate);
}

#[test]
fn lifecycle_missing_completion_and_legacy_control_cannot_prove_stopped() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let fixture = lifecycle_fixture();
    for (observation, shutdown) in [
        (None, Some(&fixture.shutdown)),
        (Some(&fixture.observation), None),
        (None, None),
    ] {
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            is_duplicate: true,
            shutdown_observation: observation,
            shutdown_receipt: shutdown,
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
        assert!(outcome.is_duplicate);
    }
}

#[test]
fn lifecycle_conflicting_completion_bindings_are_unresolved() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let fixture = lifecycle_fixture();
    let wrong_ref = test_ref("conflicting-completion");
    for field in ["run", "startup", "shutdown", "control"] {
        let observation = lifecycle_observation(
            if field == "run" { &wrong_ref } else { &fixture.run_ref },
            if field == "startup" {
                &wrong_ref
            } else {
                &fixture.startup_ref
            },
            if field == "shutdown" {
                &wrong_ref
            } else {
                &fixture.shutdown.receipt_ref
            },
            if field == "control" {
                &wrong_ref
            } else {
                &fixture.control.receipt_ref
            },
        );
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            shutdown_observation: Some(&observation),
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved, "conflicting {field}");
    }
}

#[test]
fn lifecycle_active_lock_conflicts_with_completion_or_other_run() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let fixture = lifecycle_fixture();
    for (observation, shutdown) in [
        (Some(&fixture.observation), Some(&fixture.shutdown)),
        (Some(&fixture.observation), None),
        (None, Some(&fixture.shutdown)),
    ] {
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            active_run_ref: Some(&fixture.run_ref),
            shutdown_observation: observation,
            shutdown_receipt: shutdown,
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
    }
    let other_run = test_ref("other-active-run");
    let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        active_run_ref: Some(&other_run),
        shutdown_observation: None,
        shutdown_receipt: None,
        ..fixture.input()
    });
    assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
}
