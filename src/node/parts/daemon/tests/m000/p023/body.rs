fn current_run_protected_state(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut paths = vec![
        root.join(STARTUP_FILE),
        root.join(HEALTH_FILE),
        root.join(SHUTDOWN_FILE),
        root.join(CONTROL_LOCK_FILE),
        root.join(CONTROL_STOP_FILE),
        root.join(CURRENT_RUN_FILE),
    ];
    paths.extend(
        crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
            .iter()
            .map(|name| root.join(format!("receipts/adapter-shutdown-{name}.preserves"))),
    );
    paths
        .into_iter()
        .map(|path| {
            let bytes = if path.exists() {
                Some(std::fs::read(&path).expect("protected lifecycle bytes"))
            } else {
                None
            };
            (path, bytes)
        })
        .collect()
}

#[test]
fn current_run_shutdown_observation_binds_completion_and_restart_lineage() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let workspace = crate::test_support::process_workspace("node-current-run-lineage").expect("isolated workspace");
    init_local(&InitInput {
        state_root: &workspace,
        node_id: "node:lineage",
    })
    .expect("initialize");
    run_local(&RunInput { state_root: &workspace }).expect("first run");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
    let first = current_node_run_binding(&root).expect("first binding");
    let stopped = stop_local_with_root(&root).expect("current shutdown");
    let observations = read_node_run_observations(&root, &first.startup_receipt_ref).expect("completed observations");
    let control = crate::node_runtime::parse_control_receipt(&stopped.control_receipt_value).expect("shutdown control");
    let completion = observations.shutdown_observation.as_ref().expect("run-bound completion");
    assert_eq!(completion.run_ref, first.run_ref);
    assert_eq!(completion.shutdown_receipt_ref, stopped.shutdown_ref);
    assert_eq!(completion.control_receipt_ref, stopped.control_receipt_ref);
    assert_eq!(
        observations.classify("shutdown", Some(&control), true).lifecycle,
        crate::node_runtime::NodeRunLifecycle::Stopped
    );
    run_local_with_root(&root).expect("restart with completed evidence");
    let second = current_node_run_binding(&root).expect("second binding");
    assert_eq!(second.previous_run_ref.as_deref(), Some(first.run_ref.as_str()));
    assert_ne!(second.run_ref, first.run_ref);
    assert!(!workspace.join(CURRENT_RUN_SHUTDOWN_FILE).exists());
    assert_eq!(require_active_lock(&root).expect("current active binding").run_ref, second.run_ref);
}

#[test]
fn current_run_observation_rejection_precedes_shutdown_effects() {
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    for damage in [
        "missing-run",
        "unparseable-run",
        "unreadable-completion",
        "conflicting-lock",
        "conflicting-completion",
    ] {
        let workspace =
            crate::test_support::process_workspace("node-current-run-rejection").expect("isolated workspace");
        init_local(&InitInput {
            state_root: &workspace,
            node_id: "node:observation-rejection",
        })
        .expect("initialize");
        run_local(&RunInput { state_root: &workspace }).expect("run node");
        let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
        let binding = current_node_run_binding(&root).expect("current binding");
        match damage {
            "missing-run" => std::fs::remove_file(workspace.join(CURRENT_RUN_FILE)).expect("remove binding"),
            "unparseable-run" => {
                std::fs::write(workspace.join(CURRENT_RUN_FILE), b"<invalid-run>").expect("damage binding")
            }
            "unreadable-completion" => {
                std::fs::create_dir(workspace.join(CURRENT_RUN_SHUTDOWN_FILE)).expect("non-file observation")
            }
            "conflicting-lock" => {
                let other = local_ref("different-node-run", "conflicting").expect("other run ref");
                let value = active_lock_value(&root, &binding.startup_receipt_ref, &other).expect("conflicting lock");
                write_preserves(&root, &fixed_node_path(CONTROL_LOCK_FILE).expect("lock path"), &value)
                    .expect("replace lock");
            }
            "conflicting-completion" => {
                let other = local_ref("different-node-run", "conflicting").expect("other run ref");
                let value = crate::node_runtime::node_run_shutdown_observation_value(
                    &crate::node_runtime::NodeRunShutdownObservationValueInput {
                        run_ref: &other,
                        startup_receipt_ref: &binding.startup_receipt_ref,
                        shutdown_receipt_ref: &other,
                        control_receipt_ref: &other,
                    },
                )
                .expect("conflicting completion");
                write_preserves(&root, &fixed_node_path(CURRENT_RUN_SHUTDOWN_FILE).expect("completion path"), &value)
                    .expect("place conflicting completion");
            }
            _ => unreachable!("bounded damage cases"),
        }
        let before = current_run_protected_state(&workspace);
        let _error = stop_local_with_root(&root).expect_err("inconsistent current-run evidence rejects shutdown");
        status_local_with_root(&root).expect_err("inconsistent current-run observations reject status");
        assert_eq!(current_run_protected_state(&workspace), before, "lifecycle changed after {damage}");
        assert!(!workspace.join(SHUTDOWN_FILE).exists());
        for name in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
            assert!(!workspace.join(format!("receipts/adapter-shutdown-{name}.preserves")).exists());
        }
    }
}

#[test]
fn legacy_completion_without_run_binding_cannot_report_stopped_or_restart() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    for missing in [CURRENT_RUN_SHUTDOWN_FILE, CURRENT_RUN_FILE] {
        let workspace =
            crate::test_support::process_workspace("node-historical-completion-only").expect("isolated workspace");
        init_local(&InitInput {
            state_root: &workspace,
            node_id: "node:historical",
        })
        .expect("initialize");
        run_local(&RunInput { state_root: &workspace }).expect("run node");
        let stopped = stop_local(&StopInput { state_root: &workspace }).expect("complete original run");
        std::fs::remove_file(workspace.join(missing)).expect("remove current-run observation");
        let before = current_run_protected_state(&workspace);
        assert_eq!(
            crate::node_runtime::parse_node_shutdown_receipt(&stopped.shutdown_value)
                .expect("historical shutdown remains readable")
                .receipt_ref,
            stopped.shutdown_ref
        );
        assert_eq!(
            crate::node_runtime::parse_control_receipt(&stopped.control_receipt_value)
                .expect("historical control remains readable")
                .receipt_ref,
            stopped.control_receipt_ref
        );
        status_local(&StatusInput { state_root: &workspace })
            .expect_err("legacy receipt alone cannot prove current stopped");
        run_local(&RunInput { state_root: &workspace }).expect_err("missing run lineage cannot reset the run identity");
        assert_eq!(current_run_protected_state(&workspace), before);
        assert!(!workspace.join(CONTROL_LOCK_FILE).exists());
    }
}

#[test]
fn denied_repeat_stop_preserves_completed_run_status_and_restart() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let workspace = crate::test_support::process_workspace("node-repeat-stop-readback").expect("isolated workspace");
    init_local(&InitInput {
        state_root: &workspace,
        node_id: "node:repeat-stop",
    })
    .expect("initialize");
    run_local(&RunInput { state_root: &workspace }).expect("run node");
    let stopped = stop_local(&StopInput { state_root: &workspace }).expect("complete current run");
    stop_local(&StopInput { state_root: &workspace }).expect_err("repeat stop remains denied");
    assert_eq!(status_local(&StatusInput { state_root: &workspace }).expect("completed status").status, "stopped");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("node capability");
    let startup = current_startup_receipt(&root).expect("completed startup");
    let observations = read_node_run_observations(&root, &startup.receipt_ref).expect("completed observations");
    assert_eq!(
        completed_node_run_control(&root, &observations)
            .expect("immutable control")
            .expect("completion control")
            .receipt_ref,
        stopped.control_receipt_ref
    );
    run_local_with_root(&root).expect("restart retains successful completion despite later denial");
    assert_eq!(status_local_with_root(&root).expect("restarted status").status, "running");
}

#[test]
fn service_propagates_unresolved_historical_shutdown_without_stopping_current_run() {
    // r[verify molten.audit_f14.duplicate_scope]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let workspace =
        crate::test_support::process_workspace("node-service-historical-shutdown").expect("isolated workspace");
    init_local(&InitInput {
        state_root: &workspace,
        node_id: "node:service-historical",
    })
    .expect("initialize");
    run_local(&RunInput { state_root: &workspace }).expect("first run");
    let request = shutdown_request().expect("shutdown request");
    submit_and_dispatch(&workspace, &request.value);
    let old_completion = std::fs::read(workspace.join(CURRENT_RUN_SHUTDOWN_FILE)).expect("first completion");
    run_local(&RunInput { state_root: &workspace }).expect("restart");
    std::fs::write(workspace.join(CURRENT_RUN_SHUTDOWN_FILE), old_completion).expect("restore historical observation");
    let active_lock = std::fs::read(workspace.join(CONTROL_LOCK_FILE)).expect("current active lock");
    submit_control_request(&ControlSubmitInput {
        state_root: &workspace,
        request_value: &request.value,
    })
    .expect("queue duplicate");
    let served = serve_control(&ControlServeInput {
        state_root: &workspace,
        topic: DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("service records unresolved loop");
    assert_eq!(served.decision, "deny");
    assert!(!served.has_stopped);
    assert_eq!(std::fs::read(workspace.join(CONTROL_LOCK_FILE)).expect("preserved lock"), active_lock);
    assert!(!workspace.join(SHUTDOWN_FILE).exists());
}

#[test]
fn service_preserves_completed_shutdown_when_pending_work_denies_the_loop() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let workspace =
        crate::test_support::process_workspace("node-service-completed-with-pending").expect("isolated workspace");
    init_local(&InitInput {
        state_root: &workspace,
        node_id: "node:completed-with-pending",
    })
    .expect("initialize");
    run_local(&RunInput { state_root: &workspace }).expect("run node");
    let policies = [local_ref("node-control-policy", "shutdown").expect("policy")];
    let resources = [local_ref("node-control-resource", "shutdown").expect("resource")];
    for suffix in ["first-pending", "second-pending"] {
        let authority = [local_ref("node-control-authority", suffix).expect("authority")];
        let request = shutdown_request_with_refs(&authority, &policies, &resources);
        submit_control_request(&ControlSubmitInput {
            state_root: &workspace,
            request_value: &request.value,
        })
        .expect("queue distinct shutdown");
    }
    let served = serve_control(&ControlServeInput {
        state_root: &workspace,
        topic: DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("service observes completed shutdown and pending work");
    assert_eq!(served.decision, "deny");
    assert!(served.has_stopped, "pending-work denial cannot erase current-run shutdown completion");
    assert!(!workspace.join(CONTROL_LOCK_FILE).exists());
    assert!(workspace.join(CURRENT_RUN_SHUTDOWN_FILE).is_file());
}
