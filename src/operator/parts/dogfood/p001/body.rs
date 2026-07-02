
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub promotion_summary_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNodeDogfoodInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNodeDogfoodRun {
    pub decision: String,
    pub workflow_ref: String,
    pub workflow_value: IoValue,
    pub step_values: Vec<IoValue>,
    pub checkpoint_values: Vec<IoValue>,
    pub report_ref: String,
    pub report_value: IoValue,
    pub release_gate_ref: Option<String>,
    pub release_gate_value: Option<IoValue>,
    pub replay_verify_ref: Option<String>,
    pub replay_verify_value: Option<IoValue>,
    pub replay_index_ref: Option<String>,
    pub replay_index_value: Option<IoValue>,
    pub ledger_import_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStep {
    pub step_ref: String,
    pub name: String,
    pub request_ref: Option<String>,
    pub receipt_ref: Option<String>,
    pub decision: String,
    pub replay_status: String,
    pub mandatory: bool,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorWorkflow {
    pub workflow_ref: String,
    pub workflow_id: String,
    pub steps: Vec<OperatorStep>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub replay_profile: String,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorCheckpoint {
    pub checkpoint_ref: String,
    pub workflow_id: String,
    pub sequence: u64,
    pub step_ref: String,
    pub request_ref: Option<String>,
    pub receipt_ref: Option<String>,
    pub result_ref: Option<String>,
    pub state_root_ref: String,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogfoodReport {
    pub report_ref: String,
    pub decision: String,
    pub workflow_ref: String,
    pub checkpoint_refs: Vec<String>,
    pub step_receipts: Vec<(String, String)>,
    pub gate_receipts: Vec<String>,
    pub repro_bundles: Vec<String>,
    pub final_state_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

pub fn operator_step_value(input: &OperatorStepInput<'_>) -> Result<IoValue> {
    validate_step_name(input.name)?;
    validate_decision(input.decision)?;
    validate_replay_status(input.replay_status)?;
    validate_optional_ref(input.request_ref, "operator step request ref")?;
    validate_optional_ref(input.receipt_ref, "operator step receipt ref")?;
    validate_refs(input.artifact_refs, "operator step artifact ref")?;
    ensure_count_at_most(input.artifact_refs.len(), MAX_OPERATOR_REFS, "operator step artifact refs")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_OPERATOR_DIAGNOSTICS, "operator step diagnostics")?;
    let has_receipt = input.receipt_ref.is_some();
    let mandatory_status = if input.mandatory { "pass" } else { "diagnostic" };
    Ok(crate::preserves_rail::record("operator-step-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_STEP_SCHEMA),
        crate::preserves_rail::record("name", vec![crate::preserves_rail::string(input.name)]),
        crate::preserves_rail::record("request", vec![optional_ref_value(input.request_ref)]),
        crate::preserves_rail::record("receipt", vec![optional_ref_value(input.receipt_ref)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("replay", vec![crate::preserves_rail::string(input.replay_status)]),
        crate::preserves_rail::record("mandatory", vec![crate::preserves_rail::bool_value(input.mandatory)]),
        crate::preserves_rail::record("artifacts", vec![refs_sequence(input.artifact_refs)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-step", "pass"),
            ("explicit-request-ref", status(input.request_ref.is_some())),
            ("canonical-receipt-ref", status(has_receipt)),
            ("mandatory-classification", mandatory_status),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_operator_step(value: &IoValue) -> Result<OperatorStep> {
    let fields = value
        .collect_simple_record("operator-step-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-step-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_STEP_SCHEMA, "operator step")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-step", "operator step")?;
    require_check(&checks, "no-text-oracle", "operator step")?;
    let name = record_string(&fields[1], "name")?;
    let request_ref = record_optional_ref(&fields[2], "request")?;
    let receipt_ref = record_optional_ref(&fields[3], "receipt")?;
    let decision = record_string(&fields[4], "decision")?;
    let replay_status = record_string(&fields[5], "replay")?;
    let is_mandatory = record_bool(&fields[6], "mandatory")?;
    let artifact_refs = record_ref_sequence(&fields[7], "artifacts")?;
    let diagnostics = record_string_sequence(&fields[8], "diagnostics")?;
    validate_step_name(&name)?;
    validate_decision(&decision)?;
    validate_replay_status(&replay_status)?;
    Ok(OperatorStep {
        step_ref: crate::preserves_rail::canonical_hash(value)?,
        name,
        request_ref,
        receipt_ref,
        decision,
        replay_status,
        mandatory: is_mandatory,
        artifact_refs,
        diagnostics,
        checks,
        value: value.clone(),
    })
}

pub fn operator_checkpoint_value(input: &OperatorCheckpointInput<'_>) -> Result<IoValue> {
    validate_workflow_id(input.workflow_id)?;
    validate_ref(input.step_ref, "operator checkpoint step ref")?;
    validate_optional_ref(input.request_ref, "operator checkpoint request ref")?;
    validate_optional_ref(input.receipt_ref, "operator checkpoint receipt ref")?;
    validate_optional_ref(input.result_ref, "operator checkpoint result ref")?;
    validate_ref(input.state_root_ref, "operator checkpoint state root ref")?;
    Ok(crate::preserves_rail::record("operator-checkpoint-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_CHECKPOINT_SCHEMA),
        crate::preserves_rail::record("workflow", vec![crate::preserves_rail::string(input.workflow_id)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(input.sequence)]),
        crate::preserves_rail::record("step", vec![crate::preserves_rail::string(input.step_ref)]),
        crate::preserves_rail::record("request", vec![optional_ref_value(input.request_ref)]),
        crate::preserves_rail::record("receipt", vec![optional_ref_value(input.receipt_ref)]),
        crate::preserves_rail::record("result", vec![optional_ref_value(input.result_ref)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(input.state_root_ref)]),
        checks_value_from_pairs(&[
            ("checkpoint-after-step", "pass"),
            ("request-receipt-result-bound", status(input.receipt_ref.is_some() && input.result_ref.is_some())),
            ("explicit-state-root", "pass"),
        ]),
    ]))
}

pub fn parse_operator_checkpoint(value: &IoValue) -> Result<OperatorCheckpoint> {
    let fields = value
        .collect_simple_record("operator-checkpoint-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-checkpoint-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_CHECKPOINT_SCHEMA, "operator checkpoint")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "checkpoint-after-step", "operator checkpoint")?;
    require_check(&checks, "explicit-state-root", "operator checkpoint")?;
    let workflow_id = record_string(&fields[1], "workflow")?;
    validate_workflow_id(&workflow_id)?;
    let sequence = record_u64(&fields[2], "sequence")?;
    let step_ref = record_ref(&fields[3], "step")?;
    let request_ref = record_optional_ref(&fields[4], "request")?;
    let receipt_ref = record_optional_ref(&fields[5], "receipt")?;
    let result_ref = record_optional_ref(&fields[6], "result")?;
    let state_root_ref = record_ref(&fields[7], "state-root")?;
    Ok(OperatorCheckpoint {
        checkpoint_ref: crate::preserves_rail::canonical_hash(value)?,
        workflow_id,
        sequence,
        step_ref,
        request_ref,
        receipt_ref,
        result_ref,
        state_root_ref,
        checks,
        value: value.clone(),
    })
}

pub fn operator_workflow_value(input: &OperatorWorkflowInput<'_>) -> Result<IoValue> {
    validate_workflow_id(input.workflow_id)?;
    validate_refs(input.policy_refs, "operator workflow policy ref")?;
    validate_refs(input.capability_refs, "operator workflow capability ref")?;
    validate_refs(input.resource_refs, "operator workflow resource ref")?;
    ensure_count_at_most(input.steps.len(), MAX_OPERATOR_STEPS, "operator workflow steps")?;
    let steps = input.steps.iter().map(parse_operator_step).collect::<Result<Vec<_>>>()?;
    let has_mandatory_step_authority = !input.capability_refs.is_empty() && !input.policy_refs.is_empty();
    let has_hidden_bypass = steps
        .iter()
        .any(|step| step.mandatory && (step.request_ref.is_none() || step.receipt_ref.is_none()));
    Ok(crate::preserves_rail::record("operator-workflow-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_WORKFLOW_SCHEMA),
        crate::preserves_rail::record("workflow-id", vec![crate::preserves_rail::string(input.workflow_id)]),
        crate::preserves_rail::record("steps", vec![crate::preserves_rail::sequence(input.steps.to_vec())]),
        crate::preserves_rail::record("policy", vec![refs_sequence(input.policy_refs)]),
        crate::preserves_rail::record("capability", vec![refs_sequence(input.capability_refs)]),
        crate::preserves_rail::record("resource", vec![refs_sequence(input.resource_refs)]),
        crate::preserves_rail::record("replay-profile", vec![crate::preserves_rail::string(input.replay_profile)]),
        checks_value_from_pairs(&[
            ("canonical-workflow", "pass"),
            ("no-hidden-bypass", status(!has_hidden_bypass)),
            ("explicit-operator-authority", status(has_mandatory_step_authority)),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_operator_workflow(value: &IoValue) -> Result<OperatorWorkflow> {
    let fields = value
        .collect_simple_record("operator-workflow-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-workflow-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_WORKFLOW_SCHEMA, "operator workflow")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-workflow", "operator workflow")?;
    require_check(&checks, "no-hidden-bypass", "operator workflow")?;
    require_check(&checks, "no-text-oracle", "operator workflow")?;
    let step_values = record_iovalue_sequence(&fields[2], "steps")?;
    ensure_count_at_most(step_values.len(), MAX_OPERATOR_STEPS, "operator workflow steps")?;
    let steps = step_values.iter().map(parse_operator_step).collect::<Result<Vec<_>>>()?;
    Ok(OperatorWorkflow {
        workflow_ref: crate::preserves_rail::canonical_hash(value)?,
        workflow_id: record_string(&fields[1], "workflow-id")?,
        steps,
        policy_refs: record_ref_sequence(&fields[3], "policy")?,
        capability_refs: record_ref_sequence(&fields[4], "capability")?,
        resource_refs: record_ref_sequence(&fields[5], "resource")?,
        replay_profile: record_string(&fields[6], "replay-profile")?,
        checks,
        value: value.clone(),
    })
}

struct ReportParts {
    checkpoint_refs: Vec<String>,
    step_receipts: Vec<(String, String)>,
    diagnostics: Vec<String>,
}
