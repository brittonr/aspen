//! Service tests that carry a completed or unresolved shutdown into the next run.

#[test]
fn service_propagates_unresolved_historical_shutdown_without_stopping_current_run() {
    // r[verify molten.audit_f14.duplicate_scope]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let workspace =
        crate::test_support::process_workspace("node-service-historical-shutdown").expect("isolated workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &workspace,
        node_id: "node:service-historical",
    })
    .expect("initialize");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &workspace })
        .expect("first run");
    let request = super::super::running::shutdown_request().expect("shutdown request");
    super::mailbox::submit_and_dispatch(&workspace, &request.value);
    let old_completion =
        std::fs::read(workspace.join(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE)).expect("first completion");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &workspace })
        .expect("restart");
    std::fs::write(workspace.join(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE), old_completion)
        .expect("restore historical observation");
    let active_lock =
        std::fs::read(workspace.join(super::super::settings::CONTROL_LOCK_FILE)).expect("current active lock");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &workspace,
        request_value: &request.value,
    })
    .expect("queue duplicate");
    let served = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &workspace,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("service records unresolved loop");
    assert_eq!(served.decision, "deny");
    assert!(!served.has_stopped);
    assert_eq!(
        std::fs::read(workspace.join(super::super::settings::CONTROL_LOCK_FILE)).expect("preserved lock"),
        active_lock
    );
    assert!(!workspace.join(super::super::settings::SHUTDOWN_FILE).exists());
}

#[test]
fn service_preserves_completed_shutdown_when_pending_work_denies_the_loop() {
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.validation]
    let workspace =
        crate::test_support::process_workspace("node-service-completed-with-pending").expect("isolated workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &workspace,
        node_id: "node:completed-with-pending",
    })
    .expect("initialize");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &workspace })
        .expect("run node");
    let policies = [super::super::storage::local_ref("node-control-policy", "shutdown").expect("policy")];
    let resources = [super::super::storage::local_ref("node-control-resource", "shutdown").expect("resource")];
    for suffix in ["first-pending", "second-pending"] {
        let authority = [super::super::storage::local_ref("node-control-authority", suffix).expect("authority")];
        let request = super::halts::shutdown_request_with_refs(&authority, &policies, &resources);
        super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
            state_root: &workspace,
            request_value: &request.value,
        })
        .expect("queue distinct shutdown");
    }
    let served = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &workspace,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("service observes completed shutdown and pending work");
    assert_eq!(served.decision, "deny");
    assert!(served.has_stopped, "pending-work denial cannot erase current-run shutdown completion");
    assert!(!workspace.join(super::super::settings::CONTROL_LOCK_FILE).exists());
    assert!(workspace.join(super::super::tracking::CURRENT_RUN_SHUTDOWN_FILE).is_file());
}
