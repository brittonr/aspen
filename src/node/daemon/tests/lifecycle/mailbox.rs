//! Control inbox dispatch tests and their helpers.

#[test]
fn control_inbox_dispatch_imports_receipts_and_denies_missing_operation_payloads() {
    let root = initialized_control_root("node-control-socket", "node:control");
    let status = assert_status_dispatch(&root);
    assert_missing_payload_denied(&root, &status);
    assert_missing_authority_denied(&root, &status);
    assert_shutdown_dispatch(&root);
    assert_dispatch_requires_lock(&root);
}

pub(super) fn initialized_control_root(label: &str, node_id: &str) -> std::path::PathBuf {
    let root = super::fixtures::temp_dir(label);
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id,
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    root
}

pub(super) fn submit_and_dispatch(
    root: &std::path::Path,
    request_value: &preserves::IOValue,
) -> super::super::outcomes::ControlDispatch {
    let submitted = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: root,
        request_value,
    })
    .expect("submit request");
    let state_root = crate::node_state::DirectoryRoot::open(root).expect("open node state root");
    let inbox = state_root.control_inbox().expect("open control inbox");
    assert!(
        inbox
            .try_exists(&crate::node_state::RelativePath::parse(&submitted.inbox_entry).expect("inbox entry"))
            .expect("inspect inbox entry")
    );
    super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
        state_root: root,
        request_entry: Some(&submitted.inbox_entry),
    })
    .expect("dispatch request")
}

fn assert_status_dispatch(root: &std::path::Path) -> crate::node_runtime::ControlRequest {
    let request = super::super::running::status_request().expect("status request");
    let dispatched = submit_and_dispatch(root, &request.value);
    assert_eq!(dispatched.operation, "status");
    let receipt =
        crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.request_ref, request.request_ref);
    assert_ledger_contains(root, &[
        "node-control-request",
        "node-control-queue-receipt",
        "node-health-receipt",
        "node-control-receipt",
    ]);
    request
}

fn assert_ledger_contains(root: &std::path::Path, expected: &[&str]) {
    let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
        .expect("list ledger")
        .into_iter()
        .map(|entry| entry.artifact_kind)
        .collect::<Vec<_>>();
    for expected_kind in expected {
        assert!(kinds.iter().any(|kind| kind.as_str() == *expected_kind), "missing ledger kind {expected_kind}");
    }
}

fn assert_missing_payload_denied(root: &std::path::Path, status: &crate::node_runtime::ControlRequest) {
    let target_ref = super::super::storage::local_ref("install-target", "fixture").expect("target ref");
    let install_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "install",
        target_ref: Some(&target_ref),
        payload_ref: None,
        authority_refs: &status.authority_refs,
        policy_refs: &status.policy_refs,
        resource_refs: &status.resource_refs,
        evidence_refs: &[],
    })
    .expect("install request");
    let dispatch = submit_and_dispatch(root, &install_value);
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("install receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires payload ref")));
}

fn assert_missing_authority_denied(root: &std::path::Path, status: &crate::node_runtime::ControlRequest) {
    let missing_authority =
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &[],
            policy_refs: &status.policy_refs,
            resource_refs: &status.resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority request");
    let dispatch = submit_and_dispatch(root, &missing_authority);
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("missing receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));
}

fn assert_shutdown_dispatch(root: &std::path::Path) {
    let request = super::super::running::shutdown_request().expect("shutdown request");
    let dispatch = submit_and_dispatch(root, &request.value);
    let receipt =
        crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("shutdown receipt");
    assert_eq!(receipt.decision, "pass");
    assert!(!root.join(super::super::settings::CONTROL_LOCK_FILE).exists());
}

fn assert_dispatch_requires_lock(root: &std::path::Path) {
    let error =
        super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
            state_root: root,
            request_entry: None,
        })
        .expect_err("dispatch requires lock");
    assert!(error.to_string().contains("active node lock"));
}
