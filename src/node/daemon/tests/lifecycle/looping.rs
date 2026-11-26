//! Control loop idempotency and duplicate request tests.

#[test]
fn compatibility_request_path_rejects_directory_authority() {
    // r[verify molten.node.cap_std_request_lifecycle]
    let root = super::fixtures::temp_dir("node-control-compatibility-path-deny");
    let error = super::super::submission::dispatch_control_request(&super::super::requests::ControlDispatchInput {
        state_root: &root,
        request_path: Some(std::path::Path::new("../outside.preserves")),
    })
    .expect_err("directory-bearing compatibility path must deny");
    assert!(error.to_string().contains("selected state root inbox"));
}

#[test]
fn control_loop_processes_queue_idempotently_and_stops_on_shutdown() {
    let root = super::fixtures::temp_dir("node-control-loop");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:loop",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let status_request = super::super::running::status_request().expect("status request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &status_request.value,
    })
    .expect("submit status");
    let first_loop = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: 1,
    })
    .expect("run one status request");
    assert_eq!(first_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
    assert!(!first_loop.has_stopped);
    assert_eq!(crate::ledger::artifact_kind(&first_loop.loop_receipt_value), "node-control-loop-receipt");
    assert_eq!(crate::ledger::artifact_kind(&first_loop.heartbeat_receipt_value), "node-control-heartbeat-receipt");

    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &status_request.value,
    })
    .expect("resubmit duplicate status");
    let duplicate_loop = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: 1,
    })
    .expect("run duplicate status request");
    assert_eq!(duplicate_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
    assert_eq!(duplicate_loop.dispatch_receipt_refs, first_loop.dispatch_receipt_refs);

    let shutdown_request = super::super::running::shutdown_request().expect("shutdown request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &shutdown_request.value,
    })
    .expect("submit shutdown");
    let shutdown_loop = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: super::super::settings::DEFAULT_CONTROL_LOOP_REQUESTS,
    })
    .expect("run shutdown request");
    assert!(shutdown_loop.has_stopped);
    assert!(!root.join(super::super::settings::CONTROL_LOCK_FILE).exists());
    let after_stop = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: 1,
    })
    .expect_err("stopped node loop denied");
    assert!(after_stop.to_string().contains("active node lock"));

    let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
        .expect("list loop ledger")
        .into_iter()
        .map(|entry| entry.artifact_kind)
        .collect::<Vec<_>>();
    assert!(kinds.iter().any(|kind| kind == "node-control-loop-receipt"));
    assert!(kinds.iter().any(|kind| kind == "node-control-heartbeat-receipt"));
}

#[test]
fn historical_shutdown_duplicate_preserves_current_active_run() {
    // r[verify molten.audit_f14.duplicate_scope]
    // r[verify molten.audit_f14.current_binding]
    // r[verify molten.audit_f14.preserve_state]
    // r[verify molten.audit_f14.validation]
    let root =
        crate::test_support::process_workspace("node-shutdown-current-run").expect("create isolated node workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:shutdown-current-run",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("start first run");
    let request = super::super::running::shutdown_request().expect("shutdown request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &request.value,
    })
    .expect("queue first shutdown");
    let first = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: 1,
    })
    .expect("shutdown first run");
    assert!(first.has_stopped);
    let adapter_paths = crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
        .iter()
        .map(|name| root.join(format!("receipts/adapter-shutdown-{name}.preserves")))
        .collect::<Vec<_>>();
    let adapter_receipts = adapter_paths
        .iter()
        .map(|path| std::fs::read(path).expect("first shutdown adapter receipt"))
        .collect::<Vec<_>>();
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("start second run");
    let active_lock = std::fs::read(root.join(super::super::settings::CONTROL_LOCK_FILE)).expect("second run lock");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &request.value,
    })
    .expect("queue exact historical request");
    let duplicate = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &root,
        max_requests: 1,
    })
    .expect("process historical duplicate");
    assert!(!duplicate.has_stopped, "historical shutdown cannot stop the restarted run");
    assert_eq!(duplicate.processed_request_refs, vec![request.request_ref]);
    assert_eq!(duplicate.dispatch_receipt_refs, first.dispatch_receipt_refs);
    assert_eq!(
        std::fs::read(root.join(super::super::settings::CONTROL_LOCK_FILE)).expect("preserved active lock"),
        active_lock
    );
    assert!(!root.join(super::super::settings::SHUTDOWN_FILE).exists());
    for (path, receipt) in adapter_paths.iter().zip(adapter_receipts.iter()) {
        assert_eq!(&std::fs::read(path).expect("preserved shutdown adapter receipt"), receipt);
    }
    assert_eq!(
        super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
            .expect("current status")
            .status,
        "running"
    );
}

#[test]
fn duplicate_request_with_conflicting_archive_fails_closed() {
    let root = super::fixtures::temp_dir("node-control-duplicate-conflict");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:duplicate",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let status_request = super::super::running::status_request().expect("status request");
    let submitted = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &status_request.value,
    })
    .expect("submit status");
    super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
        state_root: &root,
        request_entry: Some(&submitted.inbox_entry),
    })
    .expect("dispatch status");
    let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
    super::super::storage::write_preserves(
        &state_root,
        &super::super::paths::control_outbox_request_path(&status_request.request_ref).expect("outbox request path"),
        &crate::preserves_rail::record("tampered-node-control-request", vec![crate::preserves_rail::string(
            "conflict",
        )]),
    )
    .expect("tamper archived request");
    let duplicate = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &status_request.value,
    })
    .expect("resubmit duplicate");
    let denied =
        super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
            state_root: &root,
            request_entry: Some(&duplicate.inbox_entry),
        })
        .expect_err("conflicting duplicate denied");
    assert!(denied.to_string().contains("conflicts with archived request evidence"));
}
