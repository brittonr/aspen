use super::*;

fn test_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn parent_input() -> ClusterHarnessParentInput {
    ClusterHarnessParentInput {
        fixture_ref: test_ref("fixture"),
        command_plan_ref: test_ref("command-plan"),
        local_plan_ref: test_ref("local-plan"),
        local_run_ref: test_ref("local-run"),
        lifecycle_ref: test_ref("lifecycle"),
        drift_summary_ref: test_ref("drift"),
        cleanup_ref: test_ref("cleanup"),
        child_receipt_refs: vec![test_ref("child")],
        diagnostic_log_refs: vec![test_ref("log")],
        observed_artifact_kinds: REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS.iter().map(|kind| (*kind).to_string()).collect(),
        required_artifact_kinds: REQUIRED_CLUSTER_RUN_ARTIFACT_KINDS.iter().map(|kind| (*kind).to_string()).collect(),
        unsupported_pass_claim: false,
        diagnostics: Vec::new(),
        caveats: vec!["test evidence is fixture-scoped".to_string()],
    }
}

#[test]
fn canonical_parent_binds_children_and_required_artifact_kinds() {
    // r[verify molten.testing.receipt_first_cluster_harness.cli_receipt_surface]
    // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
    let receipt = build_cluster_harness_parent(&parent_input()).expect("parent receipt");
    let rendered = crate::preserves_rail::to_text(&receipt.value).expect("parent text");

    assert_eq!(receipt.decision, RUN_DIRECTORY_PASS);
    assert!(receipt.diagnostics.is_empty());
    assert!(rendered.contains("cluster-harness-run-v1"));
    assert!(rendered.contains("child-receipts-bound"));
}

#[test]
fn canonical_parent_denies_missing_kind_and_unsupported_pass_claim() {
    // r[verify molten.testing.fixture_driven_cluster_execution.observation_gate]
    // r[verify molten.testing.receipt_first_cluster_harness.run_artifact_directory]
    let mut input = parent_input();
    input.observed_artifact_kinds.retain(|kind| kind != DRIFT_SUMMARY_KIND);
    input.unsupported_pass_claim = true;
    let receipt = build_cluster_harness_parent(&input).expect("denied parent receipt");

    assert_eq!(receipt.decision, RUN_DIRECTORY_DENY);
    assert!(receipt.diagnostics.iter().any(|item| item == "cluster-run-unsupported-pass-claim"));
    assert!(
        receipt
            .diagnostics
            .iter()
            .any(|item| item == "cluster-run-missing-required-artifact-kind:cluster-harness-drift-summary")
    );
}

#[test]
fn child_timeout_or_orphan_and_stale_ticket_cleanup_are_denied() {
    // r[verify molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
    let child = child_process_value(&ClusterHarnessChildProcessInput {
        node_id: "node:fixture-a".to_string(),
        phase: "start".to_string(),
        command_profile_ref: test_ref("command-profile"),
        diagnostic_log_ref: test_ref("diagnostic-log"),
        exit_code: None,
        timed_out: true,
        orphaned: true,
        succeeded: false,
    })
    .expect("denied child process receipt");
    let cleanup = cleanup_value(&ClusterHarnessCleanupInput {
        child_process_refs: vec![crate::preserves_rail::canonical_hash(&child).expect("child ref")],
        stopped_node_ids: Vec::new(),
        orphaned_processes: vec!["start:node:fixture-a".to_string()],
        removed_ticket_refs: Vec::new(),
        remaining_ticket_paths: vec!["fixture-a/stale-ticket.preserves".to_string()],
        cleanup_succeeded: false,
        caveats: vec!["negative fixture".to_string()],
    })
    .expect("denied cleanup receipt");

    assert_eq!(
        artifact_decision(&child, CHILD_PROCESS_KIND).expect("child decision"),
        Some(RUN_DIRECTORY_DENY.to_string())
    );
    assert_eq!(
        artifact_decision(&cleanup, CLEANUP_KIND).expect("cleanup decision"),
        Some(RUN_DIRECTORY_DENY.to_string())
    );
}

#[test]
fn failure_bundle_is_canonical_diagnostic_evidence_not_pass_evidence() {
    // r[verify molten.testing.cluster_failure_repro_bundles.bundle_schema]
    // r[verify molten.testing.cluster_failure_repro_bundles.privacy_and_nonpass]
    let parent = build_cluster_harness_parent(&parent_input()).expect("parent receipt");
    let failure =
        crate::harness::failure_value("execute", &crate::error::MoltenError::invalid_harness("fixture failure"), vec![
            crate::preserves_rail::record("cluster-run-ref", vec![crate::preserves_rail::string(&parent.receipt_ref)]),
        ]);
    let bundle = crate::harness::failure_repro_bundle_value_with_command(&failure, &[
        "molten".to_string(),
        "cluster".to_string(),
        "harness-verify".to_string(),
    ])
    .expect("failure repro bundle");
    let bundle_ref = crate::preserves_rail::canonical_hash(&bundle).expect("bundle ref");

    crate::preserves_rail::validate_content_ref(&bundle_ref).expect("valid bundle ref");
    assert!(crate::harness::repro_bundle_report_value(&bundle).is_err());
}
