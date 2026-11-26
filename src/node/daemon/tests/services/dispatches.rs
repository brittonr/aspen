//! Operation install, run, and gate dispatch tests.

#[test]
fn control_operation_dispatch_installs_runs_and_gates_with_receipts() {
    let case = op_case();
    assert_install(&case);
    assert_gate(&case);
    assert_run(&case);
    assert_ledger(&case.root);
}

struct OpCase {
    root: std::path::PathBuf,
    authority_refs: Vec<String>,
    policy_refs: Vec<String>,
    resource_refs: Vec<String>,
}

fn op_case() -> OpCase {
    let root = super::fixtures::temp_dir("node-control-operations");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:ops",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    OpCase {
        root,
        authority_refs: vec![super::super::storage::local_ref("node-control-authority", "ops").expect("authority ref")],
        policy_refs: vec![super::super::storage::local_ref("node-control-policy", "ops").expect("policy ref")],
        resource_refs: vec![super::super::storage::local_ref("node-control-resource", "ops").expect("resource ref")],
    }
}

fn dispatch_value(case: &OpCase, value: &preserves::IOValue) -> crate::node_runtime::ControlReceipt {
    let submitted = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &case.root,
        request_value: value,
    })
    .expect("submit request");
    let request_path = case.root.join(super::super::settings::CONTROL_INBOX_DIR).join(&submitted.inbox_entry);
    let dispatched =
        super::super::submission::dispatch_control_request(&super::super::requests::ControlDispatchInput {
            state_root: &case.root,
            request_path: Some(&request_path),
        })
        .expect("dispatch request");
    crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt")
}

fn assert_install(case: &OpCase) {
    let payload_value =
        crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string("payload")]);
    let payload_ref = super::super::storage::import_artifact(&case.root, &payload_value).expect("import payload");
    let payload_provenance = crate::provenance::synthetic_reviewed_record(&payload_ref).expect("payload provenance");
    let payload_provenance_ref =
        super::super::storage::import_artifact(&case.root, &payload_provenance).expect("import payload provenance");
    let install_evidence_refs = vec![payload_provenance_ref];
    let install_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "install",
        target_ref: None,
        payload_ref: Some(&payload_ref),
        authority_refs: &case.authority_refs,
        policy_refs: &case.policy_refs,
        resource_refs: &case.resource_refs,
        evidence_refs: &install_evidence_refs,
    })
    .expect("install request");
    let install_receipt = dispatch_value(case, &install_value);
    assert_eq!(install_receipt.decision, "pass");
    let installed = crate::artifacts::list_artifacts(&case.root.join("registry"), Some("node-control-artifact"))
        .expect("list installed artifacts");
    assert_eq!(installed.len(), 1);
}

fn assert_gate(case: &OpCase) {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("gate receipt");
    let gate_ref = super::super::storage::import_artifact(&case.root, &gate_value).expect("import gate");
    let gate_target = super::super::storage::local_ref("node-control-gate-target", "ops").expect("gate target");
    let gate_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "gate",
        target_ref: Some(&gate_target),
        payload_ref: Some(&gate_ref),
        authority_refs: &case.authority_refs,
        policy_refs: &case.policy_refs,
        resource_refs: &case.resource_refs,
        evidence_refs: &[],
    })
    .expect("gate request");
    let gate_receipt = dispatch_value(case, &gate_request);
    assert_eq!(gate_receipt.decision, "pass");
    assert!(
        gate_receipt
            .subreceipt_refs
            .iter()
            .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
    );
}

fn assert_run(case: &OpCase) {
    let job_fixture = super::fixtures::install_job_fixture(&case.root);
    let execution_request_ref = super::super::storage::import_artifact(&case.root, &job_fixture.execution_request)
        .expect("import execution request");
    let admission_ref = super::super::storage::import_artifact(&case.root, &job_fixture.admission_receipt)
        .expect("import admission receipt");
    let job_provenance = crate::provenance::synthetic_reviewed_record(&job_fixture.job_ref).expect("job provenance");
    let job_provenance_ref =
        super::super::storage::import_artifact(&case.root, &job_provenance).expect("import job provenance");
    let run_evidence_refs = vec![job_provenance_ref];
    let run_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "run",
        target_ref: Some(&admission_ref),
        payload_ref: Some(&execution_request_ref),
        authority_refs: &case.authority_refs,
        policy_refs: &case.policy_refs,
        resource_refs: &case.resource_refs,
        evidence_refs: &run_evidence_refs,
    })
    .expect("run request");
    let run_receipt = dispatch_value(case, &run_request);
    assert_eq!(run_receipt.decision, "pass");
}

fn assert_ledger(root: &std::path::Path) {
    let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
        .expect("list operation ledger")
        .into_iter()
        .map(|entry| entry.artifact_kind)
        .collect::<Vec<_>>();
    assert!(kinds.iter().any(|kind| kind == "artifact-registry-receipt"));
    assert!(kinds.iter().any(|kind| kind == "provenance-record"));
    assert!(kinds.iter().any(|kind| kind == "provenance-receipt"));
    assert!(kinds.iter().any(|kind| kind == "job-execution-receipt"));
    assert!(kinds.iter().any(|kind| kind == "octet-source-gate-validation"));
    assert!(kinds.iter().any(|kind| kind == "node-control-operation-receipt"));
}

pub(super) struct JobFixture {
    pub(super) execution_request: preserves::IOValue,
    pub(super) admission_receipt: preserves::IOValue,
    pub(super) job_ref: String,
}

pub(super) struct StagePair {
    pub(super) source_ref: String,
    pub(super) map_ref: String,
}

pub(super) struct AdmissionParts {
    pub(super) receipt_value: preserves::IOValue,
    pub(super) receipt_ref: String,
    pub(super) stage_order: Vec<String>,
    pub(super) policy_refs: Vec<String>,
    pub(super) capability_refs: Vec<String>,
    pub(super) resource_refs: Vec<String>,
}
