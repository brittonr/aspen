//! Admission regressions use in-memory, prevalidated checker observations; no checker is mocked as
//! an executable.

#[path = "evidence/support.rs"]
mod support;

// r[verify molten.testing.live_reliability_evidence]
#[test]
fn complete_valid_run_has_canonical_external_receipt_without_authority() {
    let (expected, evidence) = support::fixture();
    let receipt = super::import(&expected, &evidence).expect("complete run");
    assert_eq!(receipt.status, "bounded-valid");
    assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("canonical ref"));
    assert_eq!(
        crate::live_reliability::comparison::admit_claim("release"),
        Err(crate::live_reliability::comparison::Issue::Claim)
    );
}

// r[verify molten.testing.live_reliability_evidence]
#[test]
fn receipt_storage_refuses_substituted_identity_and_preserves_canonical_bytes() {
    let (expected, evidence) = support::fixture();
    let receipt = super::import(&expected, &evidence).expect("admitted fixture receipt");
    let workspace = crate::test_support::process_workspace("live-evidence-ledger").expect("owned workspace");
    let root = crate::ledger::open_capability_ledger_root(workspace.as_ref()).expect("open test ledger");
    let mut substituted = receipt.clone();
    substituted.receipt_ref = support::reference("different-receipt");
    assert!(super::persist(&root, &substituted).is_err());
    assert!(crate::ledger::read_artifact_with_root(&root, &receipt.receipt_ref).is_err());
    assert!(crate::ledger::read_artifact_with_root(&root, &substituted.receipt_ref).is_err());

    let imported = super::persist(&root, &receipt).expect("persist admitted fixture");
    assert_eq!(imported.artifact_ref, receipt.receipt_ref);
    let recovered =
        crate::ledger::read_artifact(workspace.as_ref(), &imported.artifact_ref).expect("reopen the receipt");
    assert_eq!(recovered, receipt.value);
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn invalid_stale_or_lost_write_read_retains_failure_witness() {
    let (mut expected, mut evidence) = support::fixture();
    let read = evidence.recovery.last_mut().expect("last endpoint");
    read.value = serde_json::json!({"type":"integer","value":2});
    let attempt = read.attempt_id.clone();
    let completion = evidence.history["events"]
        .as_array_mut()
        .expect("events")
        .iter_mut()
        .find(|e| e["kind"] == "complete" && e["attempt_id"] == attempt)
        .expect("completion");
    completion["output"] = read.value.clone();
    evidence.report["verdict"] = serde_json::json!("invalid");
    evidence.report["invalid"] =
        serde_json::json!({"remaining_attempt_ids":[attempt],"model_state":"null","failure_class":"stale-read"});
    evidence.report["history_id"] = serde_json::json!(
        crate::live_reliability::checker::semantic_history_id(
            &serde_json::from_value(evidence.history.clone()).expect("history")
        )
        .expect("history id")
    );
    expected.checked = support::checked(&evidence);
    let receipt = super::import(&expected, &evidence).expect("bounded failure retained");
    assert_eq!(receipt.verdict, "invalid");
    assert_eq!(receipt.status, "external-failure");
}

#[test]
fn changed_source_artifact_unexecuted_report_and_duplicate_operations_fail_closed() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.identities.insert("artifact".into(), support::reference("stale artifact"));
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Identity));
    evidence.identities = expected.identities.clone();
    evidence.report["verdict"] = serde_json::json!("invalid");
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Checker));
    evidence.report = expected.checked.report.clone();
    let events = evidence.history["events"].as_array_mut().expect("events");
    events.push(events[2].clone());
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::History));
}

// r[verify molten.consensus.live_linearizability]
#[test]
fn selected_reference_checker_cannot_be_omitted() {
    let (mut expected, mut evidence) = support::fixture();
    expected.identities.insert("reference-checker".into(), support::reference("reference checker"));
    evidence.identities = expected.identities.clone();

    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Checker));
}

#[test]
fn unknown_bound_and_reference_disagreement_never_promote() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.report["verdict"] = serde_json::json!("unknown");
    evidence.report["unknown"] = serde_json::json!({"class":"max_states","bound_name":"max_states","bound_value":100,"observed":101,"message":"bound"});
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).expect("unknown retained").status, "inconclusive");
    evidence.reference_report = Some(evidence.report.clone());
    evidence.reference_report.as_mut().expect("reference")["verdict"] = serde_json::json!("valid");
    evidence.identities.insert("reference-checker".into(), support::reference("reference checker"));
    expected.identities = evidence.identities.clone();
    let mut reference_checked = support::checked(&evidence);
    reference_checked.report = evidence.reference_report.clone().expect("reference");
    reference_checked.checker_ref = evidence.identities["reference-checker"].clone();
    expected.reference_checked = Some(reference_checked);
    assert_eq!(super::import(&expected, &evidence).expect("disagreement retained").status, "checker-disagreement");
}

// r[verify molten.testing.live_reliability_claim_boundary]
#[test]
fn all_four_profiles_keep_their_own_scope_and_no_substitution() {
    let profiles: Vec<_> = [
        crate::live_reliability::comparison::Environment::Simulation,
        crate::live_reliability::comparison::Environment::NixosVm,
        crate::live_reliability::comparison::Environment::ChaoscontrolKvm,
        crate::live_reliability::comparison::Environment::OnixosLive,
    ]
    .into_iter()
    .map(|environment| crate::live_reliability::comparison::ProfileEvidence {
        environment,
        receipt_ref: support::reference("receipt"),
        corpus_ref: support::reference("corpus"),
        invariant_ref: support::reference("invariant"),
        adapter_ref: support::reference("adapter"),
        fault_mechanism: "none".into(),
        time_source: "controller".into(),
        checker_ref: support::reference("checker"),
        verdict: "valid".into(),
        non_claims: super::NON_CLAIMS.iter().map(|v| (*v).into()).collect(),
    })
    .collect();
    let comparison = crate::live_reliability::comparison::compare(&profiles).expect("distinct scopes");
    assert_eq!(comparison.profiles, profiles);
    assert!(!comparison.authority);
    let mut duplicate = profiles.clone();
    duplicate[3].environment = duplicate[0].environment;
    assert_eq!(
        crate::live_reliability::comparison::compare(&duplicate).err(),
        Some(crate::live_reliability::comparison::Issue::DuplicateEnvironment)
    );
}

#[test]
fn valid_disagreement_with_missing_witness_or_overbound_report_fails_closed() {
    let (mut expected, mut evidence) = support::fixture();
    evidence.report["verdict"] = serde_json::json!("invalid");
    expected.checked = support::checked(&evidence);
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Witness));
    evidence.report["verdict"] = serde_json::json!("valid");
    expected.checked = support::checked(&evidence);
    expected.max_witness_bytes = 1;
    assert_eq!(super::import(&expected, &evidence).err(), Some(super::Issue::Witness));
}

include!("evidence/faultrecoverytests.rs");
