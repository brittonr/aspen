
fn evidence_summary(value: &IoValue) -> Option<String> {
    if let Ok(evidence) = parse_nix_dogfood_evidence(value) {
        return Some(format!(
            "operator Nix dogfood evidence ref={} output={} report={} release_gate={} nextest={} (summary is non-normative)",
            evidence.evidence_ref,
            evidence.output_path,
            evidence.report_ref,
            evidence.release_gate_ref,
            evidence.nextest_check_path
        ));
    }
    if let Ok(receipt) = parse_nix_dogfood_verify_receipt(value) {
        return Some(format!(
            "operator Nix dogfood verify receipt ref={} decision={} evidence={} report={} release_gate={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.evidence_ref,
            receipt.report_ref,
            receipt.release_gate_ref,
            receipt.diagnostics.len()
        ));
    }
    if let Ok(bundle) = parse_release_evidence_bundle(value) {
        return Some(format!(
            "operator release evidence bundle ref={} output={} report={} release_gate={} nix_verify={} members={} (summary is non-normative)",
            bundle.bundle_ref,
            bundle.output_path,
            bundle.report_ref,
            bundle.release_gate_ref,
            bundle.nix_verify_ref,
            bundle.member_refs.len()
        ));
    }
    if let Ok(receipt) = parse_release_evidence_bundle_verify_receipt(value) {
        return Some(format!(
            "operator release evidence bundle verify receipt ref={} decision={} bundle={} report={} release_gate={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.bundle_ref,
            receipt.report_ref,
            receipt.release_gate_ref,
            receipt.diagnostics.len()
        ));
    }
    None
}

fn promotion_summary(value: &IoValue) -> Option<String> {
    if let Ok(receipt) = parse_release_promotion_gate_receipt(value) {
        return Some(format!(
            "operator release promotion gate receipt ref={} decision={} bundle_verify={} key={} source={} octet={} cairn={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.bundle_verify_ref,
            receipt.selected_key_ref,
            receipt.source_ref,
            receipt.octet_ref,
            receipt.cairn_ref,
            receipt.diagnostics.len()
        ));
    }
    if let Ok(summary) = parse_release_promotion_summary(value) {
        return Some(format!(
            "operator release promotion summary ref={} decision={} promotion={} signed={} key={} source={} octet={} cairn={} diagnostics={} (summary is non-normative)",
            summary.summary_ref,
            summary.decision,
            summary.promotion_ref,
            summary.signed_envelope_ref,
            summary.signed_key_ref,
            summary.source_ref,
            summary.octet_ref,
            summary.cairn_ref,
            summary.diagnostics.len()
        ));
    }
    None
}

fn export_summary(value: &IoValue) -> Option<String> {
    if let Ok(manifest) = parse_release_export_manifest(value) {
        return Some(format!(
            "operator release export manifest ref={} promotion_summary={} members={} (summary is non-normative)",
            manifest.manifest_ref,
            manifest.promotion_summary_ref,
            manifest.member_refs.len()
        ));
    }
    if let Ok(receipt) = parse_release_export_verify_receipt(value) {
        return Some(format!(
            "operator release export verify receipt ref={} decision={} manifest={} promotion_summary={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.manifest_ref,
            receipt.promotion_summary_ref,
            receipt.diagnostics.len()
        ));
    }
    None
}

struct StepCheckpointInput<'a> {
    name: &'a str,
    request_ref: Option<&'a str>,
    receipt_ref: Option<&'a str>,
    result_ref: Option<&'a str>,
    decision: &'a str,
    replay_status: &'a str,
    mandatory: bool,
    artifact_refs: &'a [String],
    diagnostics: &'a [String],
    state_root_ref: &'a str,
}

#[derive(Debug, Default)]
struct StepCheckpointBuffers {
    steps: Vec<IoValue>,
    checkpoints: Vec<IoValue>,
}

fn push_step_checkpoint(buffers: &mut StepCheckpointBuffers, input: StepCheckpointInput<'_>) -> Result<()> {
    let step = operator_step_value(&OperatorStepInput {
        name: input.name,
        request_ref: input.request_ref,
        receipt_ref: input.receipt_ref,
        decision: input.decision,
        replay_status: input.replay_status,
        mandatory: input.mandatory,
        artifact_refs: input.artifact_refs,
        diagnostics: input.diagnostics,
    })?;
    let step_ref = crate::preserves_rail::canonical_hash(&step)?;
    let sequence = usize_to_u64(buffers.checkpoints.len(), "operator checkpoint sequence")?;
    let checkpoint = operator_checkpoint_value(&OperatorCheckpointInput {
        workflow_id: LOCAL_NODE_WORKFLOW_ID,
        sequence,
        step_ref: &step_ref,
        request_ref: input.request_ref,
        receipt_ref: input.receipt_ref,
        result_ref: input.result_ref,
        state_root_ref: input.state_root_ref,
    })?;
    buffers.steps.push_limited_value(step, MAX_OPERATOR_STEPS, "operator steps")?;
    buffers.checkpoints.push_limited_value(checkpoint, MAX_OPERATOR_STEPS, "operator checkpoints")
}

fn dirty_state_report(state_root_ref: &str, diagnostic: String) -> Result<LocalNodeDogfoodRun> {
    let diagnostics = vec![diagnostic];
    let mut step_checkpoints = StepCheckpointBuffers::default();
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "clean-state",
        request_ref: Some(state_root_ref),
        receipt_ref: None,
        result_ref: None,
        decision: "deny",
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &[],
        diagnostics: &diagnostics,
        state_root_ref,
    })?;
    let policy_refs = vec![dogfood_ref("operator-policy")?];
    let capability_refs = vec![dogfood_ref("operator-capability")?];
    let resource_refs = vec![dogfood_ref("operator-resource")?];
    let workflow_value = operator_workflow_value(&OperatorWorkflowInput {
        workflow_id: LOCAL_NODE_WORKFLOW_ID,
        steps: &step_checkpoints.steps,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        resource_refs: &resource_refs,
        replay_profile: "deterministic",
    })?;
    let report_value = dogfood_report_value(&DogfoodReportInput {
        workflow_value: &workflow_value,
        checkpoint_values: &step_checkpoints.checkpoints,
        gate_receipt_refs: &[],
        repro_bundle_refs: &[],
        final_state_ref: state_root_ref,
        diagnostics: &[],
    })?;
    let report = parse_dogfood_report(&report_value)?;
    let StepCheckpointBuffers { steps, checkpoints } = step_checkpoints;
    Ok(LocalNodeDogfoodRun {
        decision: report.decision,
        workflow_ref: crate::preserves_rail::canonical_hash(&workflow_value)?,
        workflow_value,
        step_values: steps,
        checkpoint_values: checkpoints,
        report_ref: report.report_ref,
        report_value,
        release_gate_ref: None,
        release_gate_value: None,
        replay_verify_ref: None,
        replay_verify_value: None,
        replay_index_ref: None,
        replay_index_value: None,
        ledger_import_receipt_refs: Vec::new(),
    })
}

fn resolve_identity(state_root: &Path, policy_refs: &[String]) -> Result<crate::node_identity::Resolution> {
    let mut config = crate::node_identity::Config::new("node:dogfood-local", state_root.join("identity"));
    config.policy_refs = policy_refs.to_vec();
    crate::node_identity::resolve(&config)
}

fn start_node(
    identity: &crate::node_identity::Identity,
    identity_receipt_ref: &str,
    policy_refs: &[String],
    capability_refs: &[String],
    resource_refs: &[String],
) -> Result<crate::node_runtime::NodeRuntimeStart> {
    let adapter_bindings = crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
        .iter()
        .map(|adapter| crate::node_runtime::node_adapter_binding(adapter, &dogfood_ref(&format!("adapter:{adapter}"))?))
        .collect::<Result<Vec<_>>>()?;
    let state_root_ref = dogfood_ref("node-state-root")?;
    let effects_ref = dogfood_ref("effect-profile")?;
    let config_value = crate::node_runtime::node_config_value(&crate::node_runtime::ConfigValueInput {
        identity_ref: &identity.identity_ref,
        state_root_ref: &state_root_ref,
        adapters: &adapter_bindings,
        policy_refs,
        capability_refs,
        resource_refs,
        effect_profile_refs: &[effects_ref],
    })?;
    let source_gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = crate::preserves_rail::canonical_hash(&source_gate_value)?;
    crate::node_runtime::start_node_runtime(&crate::node_runtime::NodeRuntimeStartInput {
        config_value,
        identity_receipt_ref: identity_receipt_ref.to_string(),
        index_receipt_refs: vec![dogfood_ref("adapter-index")?],
        source_gate_receipt_refs: vec![source_gate_ref],
        source_gate_receipt_values: vec![source_gate_value],
        profile_metadata_refs: vec![dogfood_ref("production-profile-metadata")?],
        capability_receipt_refs: capability_refs.to_vec(),
        resource_receipt_refs: resource_refs.to_vec(),
        version_refs: vec![dogfood_ref(env!("CARGO_PKG_VERSION"))?],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JobStackInput<'a> {
    state_root: &'a Path,
    source: &'a Path,
    target: &'a Path,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JobRun {
    execution_request_ref: String,
    execution_receipt_ref: String,
    decision: String,
    diagnostics: Vec<String>,
    artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GcWorkflowInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    ledger_root: &'a Path,
    registry_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GcRun {
    object_ref: String,
    plan_ref: String,
    plan_decision: String,
    plan_diagnostics: Vec<String>,
    apply_ref: String,
    apply_decision: String,
    apply_diagnostics: Vec<String>,
    execution_ref: String,
    execution_decision: String,
    execution_diagnostics: Vec<String>,
    audit_ref: String,
    audit_decision: String,
    audit_diagnostics: Vec<String>,
    explain_ref: String,
    bundle_ref: String,
    bundle_profile_ref: String,
    bundle_verify_ref: String,
    bundle_verify_decision: String,
    bundle_diagnostics: Vec<String>,
    catalog_request_ref: String,
    catalog_receipt_ref: String,
    catalog_response_ref: String,
    catalog_decision: String,
    artifact_refs: Vec<String>,
}
