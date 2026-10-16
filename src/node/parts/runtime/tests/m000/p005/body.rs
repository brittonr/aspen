#[test]
fn lifecycle_denied_receipts_and_unbound_shutdown_are_unresolved() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let fixture = lifecycle_fixture();
    let denied_shutdown = lifecycle_shutdown(&fixture.startup_ref, "deny");
    let wrong_startup = lifecycle_shutdown(&test_ref("other-startup"), "pass");
    for shutdown in [&denied_shutdown, &wrong_startup] {
        let control = lifecycle_control(&fixture.startup_ref, "pass", std::slice::from_ref(&shutdown.receipt_ref));
        let observation =
            lifecycle_observation(&fixture.run_ref, &fixture.startup_ref, &shutdown.receipt_ref, &control.receipt_ref);
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            shutdown_observation: Some(&observation),
            shutdown_receipt: Some(shutdown),
            control_receipt: Some(&control),
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
    }
    let shutdown_refs = std::slice::from_ref(&fixture.shutdown.receipt_ref);
    let denied_control = lifecycle_control(&fixture.startup_ref, "deny", shutdown_refs);
    let wrong_startup_control = lifecycle_control(&test_ref("other-startup"), "pass", shutdown_refs);
    let unbound_control = lifecycle_control(&fixture.startup_ref, "pass", &[test_ref("other-shutdown")]);
    for control in [&denied_control, &wrong_startup_control, &unbound_control] {
        let observation = lifecycle_observation(
            &fixture.run_ref,
            &fixture.startup_ref,
            &fixture.shutdown.receipt_ref,
            &control.receipt_ref,
        );
        let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            shutdown_observation: Some(&observation),
            control_receipt: Some(control),
            ..fixture.input()
        });
        assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
    }
    let denied_active = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        active_run_ref: Some(&fixture.run_ref),
        shutdown_observation: None,
        shutdown_receipt: None,
        control_receipt: Some(&denied_control),
        ..fixture.input()
    });
    assert_eq!(denied_active.lifecycle, NodeRunLifecycle::Unresolved);
    let outcome = classify_node_run_lifecycle(&NodeRunLifecycleInput {
        operation: "status",
        ..fixture.input()
    });
    assert_eq!(outcome.lifecycle, NodeRunLifecycle::Unresolved);
}

#[test]
fn lifecycle_malformed_references_never_establish_current_state() {
    // r[verify molten.audit_f14.validation]
    let fixture = lifecycle_fixture();
    let uppercase = format!("blake3:{}", "A".repeat(64));
    let non_hex = format!("blake3:{}", "g".repeat(64));
    for malformed in ["", "blake3:short", "sha256:other", &uppercase, &non_hex] {
        let malformed_run = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            current_run_ref: malformed,
            active_run_ref: Some(malformed),
            shutdown_observation: None,
            shutdown_receipt: None,
            ..fixture.input()
        });
        assert_eq!(malformed_run.lifecycle, NodeRunLifecycle::Unresolved);
        let malformed_startup = classify_node_run_lifecycle(&NodeRunLifecycleInput {
            current_startup_receipt_ref: malformed,
            active_run_ref: Some(&fixture.run_ref),
            shutdown_observation: None,
            shutdown_receipt: None,
            ..fixture.input()
        });
        assert_eq!(malformed_startup.lifecycle, NodeRunLifecycle::Unresolved);
    }
    let mutations: [fn(&mut LifecycleFixture); 7] = [
        |value| {
            value.control.receipt_ref = "blake3:short".to_string();
            value.observation.control_receipt_ref = "blake3:short".to_string();
        },
        |value| value.control.request_ref = "blake3:short".to_string(),
        |value| {
            value.startup_ref = "blake3:short".to_string();
            value.control.startup_receipt_ref = "blake3:short".to_string();
            value.shutdown.startup_receipt_ref = "blake3:short".to_string();
            value.observation.startup_receipt_ref = "blake3:short".to_string();
        },
        |value| value.control.subreceipt_refs.push("blake3:short".to_string()),
        |value| value.observation.observation_ref = "blake3:short".to_string(),
        |value| {
            value.shutdown.receipt_ref = "blake3:short".to_string();
            value.observation.shutdown_receipt_ref = "blake3:short".to_string();
            value.control.subreceipt_refs = vec!["blake3:short".to_string()];
        },
        |value| value.shutdown.startup_receipt_ref = "blake3:short".to_string(),
    ];
    for mutate in mutations {
        let mut malformed = lifecycle_fixture();
        mutate(&mut malformed);
        assert_eq!(classify_node_run_lifecycle(&malformed.input()).lifecycle, NodeRunLifecycle::Unresolved);
    }
}

#[test]
fn lifecycle_active_observation_does_not_require_a_historical_control_receipt() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    let fixture = lifecycle_fixture();
    let input = NodeRunLifecycleInput {
        operation: "status",
        control_receipt: None,
        active_run_ref: Some(&fixture.run_ref),
        shutdown_observation: None,
        shutdown_receipt: None,
        ..fixture.input()
    };
    assert_eq!(classify_node_run_lifecycle(&input).lifecycle, NodeRunLifecycle::Active);
    assert_eq!(
        classify_node_run_lifecycle(&NodeRunLifecycleInput {
            active_run_ref: None,
            ..input
        })
        .lifecycle,
        NodeRunLifecycle::Unresolved
    );
    assert_eq!(
        classify_node_run_lifecycle(&NodeRunLifecycleInput {
            shutdown_observation: Some(&fixture.observation),
            ..input
        })
        .lifecycle,
        NodeRunLifecycle::Unresolved
    );
}
