//! Supervisor policy, lock portability, and heartbeat tests.

#[test]
fn control_supervisor_policy_recovers_stale_lock_and_bounds_shutdown() {
    let root = super::mailbox::initialized_control_root("node-control-supervisor-policy", "node:supervisor-policy");
    write_active_service_lock(&root, "stale");
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-supervisor-policy", "recover").expect("policy ref")];
    let recover_policy = recovering_policy(&policy_refs);

    let recovered = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: Some(&recover_policy),
    })
    .expect("recover stale lock");
    assert_eq!(recovered.decision, "pass");
    assert_eq!(recovered.supervisor_receipt_refs.len(), 2);
    assert!(recovered.supervisor_policy_ref.is_some());
    assert!(!root.join(super::super::settings::CONTROL_SERVICE_LOCK_FILE).exists());
    let restart_once = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: Some(&recover_policy),
    })
    .expect("allowed restart");
    assert_eq!(restart_once.decision, "pass");
    let restart_denied = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: Some(&recover_policy),
    })
    .expect("bounded restart denial");
    assert_eq!(restart_denied.decision, "deny");
    assert_eq!(restart_denied.ticks, 0);
    let restart_denied_text =
        crate::preserves_rail::to_text(&restart_denied.service_receipt_value).expect("restart denial receipt text");
    assert!(restart_denied_text.contains("restart attempts"));

    let shutdown = super::super::running::shutdown_request().expect("shutdown request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &shutdown.value,
    })
    .expect("submit shutdown");
    let tight_policy = bounded_shutdown_policy(&policy_refs);
    let stopped = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 4,
        max_requests_per_tick: 1,
        supervisor_policy_value: Some(&tight_policy),
    })
    .expect("shutdown serve");
    assert_eq!(stopped.decision, "deny");
    assert!(stopped.has_stopped);
    assert_eq!(stopped.supervisor_receipt_refs.len(), 2);
    let text = crate::preserves_rail::to_text(&stopped.service_receipt_value).expect("service receipt text");
    assert!(text.contains("exceeded supervisor bound"));
}

#[test]
fn canonical_lock_identity_is_portable_across_host_state_paths() {
    // r[verify molten.node.cap_std_validation]
    let left_path = crate::test_support::process_workspace("node_lock_portable_left").expect("left workspace");
    let right_path = crate::test_support::process_workspace("node_lock_portable_right").expect("right workspace");
    let left = crate::node_state::DirectoryRoot::open(&left_path).expect("left root");
    let right = crate::node_state::DirectoryRoot::open(&right_path).expect("right root");
    let startup_ref = super::super::storage::local_ref("node-startup", "portable-lock").expect("startup ref");
    let run_ref = super::super::storage::local_ref("node-run", "portable-lock").expect("run ref");
    let left_value = super::super::leases::active_lock_value(&left, &startup_ref, &run_ref).expect("left lock");
    let right_value = super::super::leases::active_lock_value(&right, &startup_ref, &run_ref).expect("right lock");

    assert_eq!(left_value, right_value);
    let text = crate::preserves_rail::to_text(&left_value).expect("lock text");
    assert!(!text.contains(left_path.to_string_lossy().as_ref()));
    assert!(!text.contains(right_path.to_string_lossy().as_ref()));
}

fn write_active_service_lock(root: &std::path::Path, service_suffix: &str) {
    let state_root = crate::node_state::DirectoryRoot::open(root).expect("open node state root");
    let startup = super::super::leases::current_startup_receipt(&state_root).expect("startup");
    let identity = crate::node_identity::parse_identity(
        &super::super::storage::read_preserves(
            &state_root,
            &crate::node_state::RelativePath::parse(super::super::settings::IDENTITY_FILE).expect("identity path"),
        )
        .expect("identity"),
    )
    .expect("parse identity");
    let service_run_ref =
        super::super::storage::local_ref("node-control-service-run", service_suffix).expect("service run ref");
    let lock_value = super::super::services::service_lock_value(&super::super::services::ServiceLockValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        node_id: &identity.node_id,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        service_run_ref: &service_run_ref,
    })
    .expect("service lock");
    super::super::storage::write_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(super::super::settings::CONTROL_SERVICE_LOCK_FILE)
            .expect("service lock path"),
        &lock_value,
    )
    .expect("write service lock");
}

fn recovering_policy(policy_refs: &[String]) -> preserves::IOValue {
    super::super::supervision::control_supervisor_policy_value(&super::super::requests::ControlSupervisorPolicyInput {
        max_restarts: 1,
        restart_window_ticks: 1,
        heartbeat_timeout_ticks: 1,
        shutdown_drain_ticks: 1,
        stale_lock_recovery: true,
        policy_refs,
        evidence_refs: &[],
    })
    .expect("recover policy")
}

fn bounded_shutdown_policy(policy_refs: &[String]) -> preserves::IOValue {
    super::super::supervision::control_supervisor_policy_value(&super::super::requests::ControlSupervisorPolicyInput {
        max_restarts: 0,
        restart_window_ticks: 1,
        heartbeat_timeout_ticks: 1,
        shutdown_drain_ticks: 0,
        stale_lock_recovery: false,
        policy_refs,
        evidence_refs: &[],
    })
    .expect("tight policy")
}

#[test]
fn control_service_heartbeats_continue_and_shutdown_stops() {
    let root = super::fixtures::temp_dir("node-control-service-shutdown");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:service-shutdown",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let idle = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 2,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("idle serve");
    assert_eq!(idle.decision, "pass");
    assert_eq!(idle.heartbeat_receipt_refs.len(), 2);
    assert_eq!(idle.loop_receipt_refs.len(), 2);
    assert!(!idle.has_stopped);

    let shutdown = super::super::running::shutdown_request().expect("shutdown request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &shutdown.value,
    })
    .expect("submit shutdown");
    let stopped = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 4,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("shutdown serve");
    assert_eq!(stopped.decision, "pass");
    assert!(stopped.has_stopped);
    assert_eq!(stopped.processed_request_refs.len(), 1);
    assert!(!root.join(super::super::settings::CONTROL_LOCK_FILE).exists());
    assert!(!root.join(super::super::settings::CONTROL_SERVICE_LOCK_FILE).exists());
}
