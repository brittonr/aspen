//! Shutdown admission and adapter close-order tests.

pub(super) fn shutdown_request_with_refs(
    authority_refs: &[String],
    policy_refs: &[String],
    resource_refs: &[String],
) -> crate::node_runtime::ControlRequest {
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "shutdown",
        target_ref: None,
        payload_ref: None,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs: &[],
    })
    .expect("shutdown request");
    crate::node_runtime::parse_control_request(&request_value).expect("parse shutdown request")
}

fn lifecycle_evidence(root: &std::path::Path) -> (Vec<u8>, bool, bool, usize) {
    let startup_bytes = std::fs::read(root.join(super::super::settings::STARTUP_FILE)).expect("startup bytes");
    let has_lock = root.join(super::super::settings::CONTROL_LOCK_FILE).exists();
    let has_shutdown = root.join(super::super::settings::SHUTDOWN_FILE).exists();
    let adapter_shutdown_receipts = std::fs::read_dir(root.join("receipts"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("adapter-shutdown-"))
                .count()
        })
        .unwrap_or(0);
    (startup_bytes, has_lock, has_shutdown, adapter_shutdown_receipts)
}

#[test]
fn shutdown_dispatch_without_authority_denies_before_effects() {
    // r[verify molten.audit_f01.preserve_state]
    // r[verify molten.audit_f01.validation]
    let root = super::mailbox::initialized_control_root("node-shutdown-denied-authority", "node:shutdown-denied");
    let before = lifecycle_evidence(&root);
    assert!(before.1, "running node holds the active lock");
    assert!(!before.2, "running node has no shutdown receipt");
    assert_eq!(before.3, 0, "running node has no adapter shutdown receipts");

    let request = shutdown_request_with_refs(
        &[],
        &[super::super::storage::local_ref("node-control-policy", "shutdown").expect("policy ref")],
        &[super::super::storage::local_ref("node-control-resource", "shutdown").expect("resource ref")],
    );
    let dispatch = super::mailbox::submit_and_dispatch(&root, &request.value);
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("deny receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));

    let after = lifecycle_evidence(&root);
    assert!(after.1, "denied shutdown preserves the active lock");
    assert!(!after.2, "denied shutdown publishes no shutdown receipt");
    assert_eq!(after.3, 0, "denied shutdown writes no adapter shutdown receipts");
    assert_eq!(after.0, before.0, "denied shutdown leaves startup evidence unchanged");
    let status = super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
        .expect("status after denial");
    assert_eq!(status.status, "running");

    let valid = super::super::running::shutdown_request().expect("valid shutdown request");
    let valid_dispatch = super::mailbox::submit_and_dispatch(&root, &valid.value);
    let valid_receipt = crate::node_runtime::parse_control_receipt(&valid_dispatch.control_receipt_value)
        .expect("valid shutdown receipt");
    assert_eq!(valid_receipt.decision, "pass");
    assert!(
        !root.join(super::super::settings::CONTROL_LOCK_FILE).exists(),
        "admitted shutdown removes the active lock"
    );
}

#[test]
fn shutdown_denials_preserve_lifecycle_state_across_rejection_cases() {
    // r[verify molten.audit_f01.preserve_state]
    // r[verify molten.audit_f01.validation]
    let root = super::mailbox::initialized_control_root("node-shutdown-denied-cases", "node:shutdown-cases");
    let authority = super::super::storage::local_ref("node-control-authority", "shutdown").expect("authority ref");
    let policy = super::super::storage::local_ref("node-control-policy", "shutdown").expect("policy ref");
    let resource = super::super::storage::local_ref("node-control-resource", "shutdown").expect("resource ref");
    struct DenialCase {
        expected_diagnostic: &'static str,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
    }
    let cases = [
        DenialCase {
            expected_diagnostic: "policy refs missing",
            authority_refs: vec![authority.clone()],
            policy_refs: Vec::new(),
            resource_refs: vec![resource.clone()],
        },
        DenialCase {
            expected_diagnostic: "resource refs missing",
            authority_refs: vec![authority.clone()],
            policy_refs: vec![policy.clone()],
            resource_refs: Vec::new(),
        },
    ];
    for case in cases {
        let before = lifecycle_evidence(&root);
        let request = shutdown_request_with_refs(&case.authority_refs, &case.policy_refs, &case.resource_refs);
        let dispatch = super::mailbox::submit_and_dispatch(&root, &request.value);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("denied receipt");
        assert_eq!(receipt.decision, "deny", "case {}", case.expected_diagnostic);
        assert!(
            receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains(case.expected_diagnostic)),
            "case {} missing diagnostic in {:?}",
            case.expected_diagnostic,
            receipt.diagnostics
        );
        let after = lifecycle_evidence(&root);
        assert!(after.1, "case {} preserves the active lock", case.expected_diagnostic);
        assert!(!after.2, "case {} publishes no shutdown receipt", case.expected_diagnostic);
        assert_eq!(after.3, 0, "case {} writes no adapter receipts", case.expected_diagnostic);
        assert_eq!(after.0, before.0, "case {} preserves startup evidence", case.expected_diagnostic);
    }
    let status = super::super::running::status_local(&super::super::requests::StatusInput { state_root: &root })
        .expect("status after denials");
    assert_eq!(status.status, "running");
}

#[test]
fn direct_stop_without_active_lock_denies_and_preserves_state() {
    // r[verify molten.audit_f01.preserve_state]
    let root = super::fixtures::temp_dir("node-shutdown-direct-denied");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:direct-denied",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let stopped = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &root })
        .expect("first stop");
    let shutdown_bytes = std::fs::read(root.join(super::super::settings::SHUTDOWN_FILE)).expect("shutdown bytes");

    let denied = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &root })
        .expect_err("second stop denied");
    assert!(denied.to_string().contains("node shutdown denied"));
    assert!(denied.to_string().contains("active node lock"));
    assert_eq!(
        std::fs::read(root.join(super::super::settings::SHUTDOWN_FILE)).expect("shutdown bytes after denial"),
        shutdown_bytes,
        "denied direct stop preserves existing successful shutdown evidence"
    );
    let denial_value = crate::preserves_rail::parse_text(
        &std::fs::read_to_string(root.join(super::super::settings::CONTROL_STOP_FILE)).expect("denial receipt text"),
    )
    .expect("parse denial receipt");
    let denial = crate::node_runtime::parse_control_receipt(&denial_value).expect("denial control receipt");
    assert_eq!(denial.decision, "deny");
    let _ = stopped;
}

#[test]
fn shutdown_effect_error_cannot_become_success() {
    // r[verify molten.audit_f01.observed_effects]
    // r[verify molten.audit_f01.validation]
    let root = super::fixtures::temp_dir("node-shutdown-effect-error");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:effect-error",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    std::fs::create_dir_all(root.join(super::super::settings::SHUTDOWN_FILE)).expect("block shutdown receipt path");
    let failed = super::super::stopping::stop_local(&super::super::requests::StopInput { state_root: &root })
        .expect_err("effect error fails stop");
    assert!(!failed.to_string().contains("pass"));
    assert!(
        root.join(super::super::settings::CONTROL_LOCK_FILE).exists(),
        "failed shutdown keeps the active lock"
    );
    assert!(
        !root.join(super::super::settings::CONTROL_STOP_FILE).exists(),
        "failed shutdown publishes no control stop receipt"
    );
}

#[test]
fn admitted_shutdown_closes_adapters_in_reverse_start_order() {
    // r[verify molten.audit_f01.observed_effects]
    let root = super::fixtures::temp_dir("node-shutdown-adapter-order");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:adapter-order",
    })
    .expect("init node");
    let run =
        super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
            .expect("run node");
    let startup = crate::node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup");
    let request = super::super::running::shutdown_request().expect("shutdown request");
    let dispatch = super::mailbox::submit_and_dispatch(&root, &request.value);
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("receipt");
    assert_eq!(receipt.decision, "pass");

    let shutdown_value = crate::preserves_rail::parse_text(
        &std::fs::read_to_string(root.join(super::super::settings::SHUTDOWN_FILE)).expect("shutdown text"),
    )
    .expect("parse shutdown receipt");
    let shutdown = crate::node_runtime::parse_node_shutdown_receipt(&shutdown_value).expect("parsed shutdown receipt");
    let mut expected_order = startup.adapters.iter().map(|adapter| adapter.name.clone()).collect::<Vec<_>>();
    expected_order.reverse();
    let actual_order = shutdown.adapters.iter().map(|adapter| adapter.name.clone()).collect::<Vec<_>>();
    assert_eq!(actual_order, expected_order);
    for adapter in shutdown.adapters.iter() {
        let receipt_path = root.join("receipts").join(format!("adapter-shutdown-{}.preserves", adapter.name));
        assert!(receipt_path.exists(), "missing adapter shutdown receipt for {}", adapter.name);
    }
}
