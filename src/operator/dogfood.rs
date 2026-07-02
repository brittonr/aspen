//! Operator dogfood workflow records and local-node runner.
//!
//! Text rendered by `operator_dogfood_summary` is a non-normative view. The
//! canonical pass/fail evidence is the Preserves report, checkpoint, and gate
//! receipt graph emitted by this module.

type IoValue = preserves::IOValue;
type Path = std::path::Path;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type SignedReceiptKey = crate::evidence::SignedReceiptKey;
type SignedReceiptKeyRevocation = crate::evidence::SignedReceiptKeyRevocation;
type VerifySignedReceiptKeyringPolicy<'a> = crate::evidence::VerifySignedReceiptKeyringPolicy<'a>;
type VerifySignedReceiptPolicy<'a> = crate::evidence::VerifySignedReceiptPolicy<'a>;

fn verify_signed_receipt_with_policy(
    value: &IoValue,
    policy: &VerifySignedReceiptPolicy<'_>,
) -> Result<crate::evidence::SignedReceipt> {
    crate::evidence::verify_signed_receipt_with_policy(value, policy)
}

fn verify_signed_receipt_with_keyring_policy(
    value: &IoValue,
    policy: &VerifySignedReceiptKeyringPolicy<'_>,
) -> Result<crate::evidence::SignedReceiptWithKey> {
    crate::evidence::verify_signed_receipt_with_keyring_policy(value, policy)
}

pub const RELEASE_EVIDENCE_SIGNING_PURPOSE: &str = "release-evidence";
pub const RELEASE_PROMOTION_SIGNING_PURPOSE: &str = "release-promotion";

const LOCAL_NODE_WORKFLOW_ID: &str = "dogfood:local-node";
const DOGFOOD_HARNESS_SUITE: &str = r#"<harness-suite-v1 "molten.harness.suite.v1" "dogfood-repro" 3
  <budget-v1 "molten.harness.budget.v1" <limits 32 8 128 65536>>
  <actor-registry-v1 "molten.harness.actor-registry.v1" [
    <actor "producer" "native">
  ]>
  <capabilities-v1 "molten.harness.capabilities.v1" [
    <grant "producer" "assert" #f "dogfood.ready">
  ]>
  [<assert "producer" "dogfood.ready">]>"#;

const MAX_OPERATOR_STEPS: usize = 64;
const MAX_OPERATOR_REFS: usize = 4096;
const MAX_OPERATOR_DIAGNOSTICS: usize = 256;
const _: () = assert!(MAX_OPERATOR_STEPS > 0);
const _: () = assert!(MAX_OPERATOR_REFS > MAX_OPERATOR_STEPS);
const _: () = assert!(MAX_OPERATOR_DIAGNOSTICS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStepInput<'a> {
    pub name: &'a str,
    pub request_ref: Option<&'a str>,
    pub receipt_ref: Option<&'a str>,
    pub decision: &'a str,
    pub replay_status: &'a str,
    pub mandatory: bool,
    pub artifact_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorCheckpointInput<'a> {
    pub workflow_id: &'a str,
    pub sequence: u64,
    pub step_ref: &'a str,
    pub request_ref: Option<&'a str>,
    pub receipt_ref: Option<&'a str>,
    pub result_ref: Option<&'a str>,
    pub state_root_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorWorkflowInput<'a> {
    pub workflow_id: &'a str,
    pub steps: &'a [IoValue],
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub replay_profile: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogfoodReportInput<'a> {
    pub workflow_value: &'a IoValue,
    pub checkpoint_values: &'a [IoValue],
    pub gate_receipt_refs: &'a [String],
    pub repro_bundle_refs: &'a [String],
    pub final_state_ref: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateInput<'a> {
    pub report_value: &'a IoValue,
    pub node_startup_ref: &'a str,
    pub node_shutdown_ref: &'a str,
    pub harness_gate_refs: &'a [String],
    pub catalog_query_refs: &'a [String],
    pub repro_verify_refs: &'a [String],
    pub replay_index_refs: &'a [String],
    pub gc_refs: &'a [String],
    pub validation_command_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub report_ref: String,
    pub startup_ref: String,
    pub shutdown_ref: String,
    pub harness_gate_refs: Vec<String>,
    pub catalog_query_refs: Vec<String>,
    pub repro_verify_refs: Vec<String>,
    pub replay_index_refs: Vec<String>,
    pub gc_refs: Vec<String>,
    pub validation_command_refs: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodEvidenceInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodEvidence {
    pub evidence_ref: String,
    pub output_path: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub file_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyInput<'a> {
    pub output_path: &'a Path,
    pub evidence_value: &'a IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub evidence_ref: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundle {
    pub bundle_ref: String,
    pub output_path: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub member_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleVerifyInput<'a> {
    pub output_path: &'a Path,
    pub bundle_value: &'a IoValue,
    pub signed_member_values: &'a [IoValue],
    pub signed_purpose: &'a str,
    pub signed_trust_root: &'a str,
    pub signed_key: &'a str,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
    pub signed_signer: Option<&'a str>,
    pub is_signed_members_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionGateInput<'a> {
    pub output_path: &'a Path,
    pub bundle_verify_value: &'a IoValue,
    pub source_evidence: &'a str,
    pub octet_evidence: &'a str,
    pub cairn_evidence: &'a str,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_trust_root: &'a str,
    pub signed_signer: Option<&'a str>,
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_verify_ref: String,
    pub bundle_ref: String,
    pub output_path_ref: String,
    pub selected_key_ref: String,
    pub source_ref: String,
    pub octet_ref: String,
    pub cairn_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

pub struct ReleasePromotionSummaryInput<'a> {
    pub output_path: &'a Path,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_trust_root: &'a str,
    pub signed_signer: Option<&'a str>,
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionSummary {
    pub summary_ref: String,
    pub decision: String,
    pub promotion_ref: String,
    pub signed_envelope_ref: String,
    pub signed_subject_ref: String,
    pub signed_key_ref: String,
    pub bundle_verify_ref: String,
    pub source_ref: String,
    pub octet_ref: String,
    pub cairn_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportManifestInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportManifest {
    pub manifest_ref: String,
    pub output_path_ref: String,
    pub promotion_summary_ref: String,
    pub member_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportVerifyInput<'a> {
    pub manifest_value: Option<&'a IoValue>,
    pub member_refs: &'a [(String, String)],
    pub archive_diagnostics: &'a [String],
}

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

impl ReportParts {
    fn collect(input: &DogfoodReportInput<'_>, workflow: &OperatorWorkflow) -> Result<Self> {
        let checkpoint_refs = input
            .checkpoint_values
            .iter()
            .map(crate::preserves_rail::canonical_hash)
            .collect::<Result<Vec<_>>>()?;
        ensure_count_at_most(checkpoint_refs.len(), MAX_OPERATOR_STEPS, "dogfood checkpoints")?;
        let diagnostics = input.diagnostics.to_vec();
        ensure_count_at_most(diagnostics.len(), MAX_OPERATOR_DIAGNOSTICS, "dogfood report diagnostics")?;
        let mut parts = Self {
            checkpoint_refs,
            step_receipts: Vec::new(),
            diagnostics,
        };
        parts.add_step_notes(workflow)?;
        parts.add_summary_notes(input, workflow)?;
        Ok(parts)
    }

    fn add_step_notes(&mut self, workflow: &OperatorWorkflow) -> Result<()> {
        for step in &workflow.steps {
            if let Some(receipt_ref) = step.receipt_ref.as_ref() {
                self.step_receipts.push_limited_value(
                    (step.name.clone(), receipt_ref.clone()),
                    MAX_OPERATOR_STEPS,
                    "dogfood step receipts",
                )?;
            }
            for diagnostic in &step.diagnostics {
                self.push_note(format!("dogfood step {} diagnostic: {diagnostic}", step.name))?;
            }
            if step.mandatory && step.receipt_ref.is_none() {
                self.push_note(format!("mandatory dogfood step {} lacks canonical receipt", step.name))?;
            }
            if step.mandatory && step.decision != "pass" {
                self.push_note(format!("mandatory dogfood step {} decision is {}", step.name, step.decision))?;
            }
            if step.mandatory && !matches!(step.replay_status.as_str(), "deterministic" | "recorded") {
                self.push_note(format!(
                    "mandatory dogfood step {} has non-release replay status {}",
                    step.name, step.replay_status
                ))?;
            }
        }
        Ok(())
    }

    fn add_summary_notes(&mut self, input: &DogfoodReportInput<'_>, workflow: &OperatorWorkflow) -> Result<()> {
        if self.checkpoint_refs.len() < workflow.steps.len() {
            self.push_note(format!(
                "dogfood workflow has {} steps but only {} checkpoints",
                workflow.steps.len(),
                self.checkpoint_refs.len()
            ))?;
        }
        if !workflow_check_pass(&workflow.checks, "no-hidden-bypass") {
            self.push_note("dogfood workflow contains hidden or unreceipted operator bypass")?;
        }
        if !workflow_check_pass(&workflow.checks, "explicit-operator-authority") {
            self.push_note("dogfood workflow lacks current explicit operator policy/capability refs")?;
        }
        if input.gate_receipt_refs.is_empty() {
            self.push_note("dogfood report requires at least one gate receipt")?;
        }
        if input.repro_bundle_refs.is_empty() {
            self.push_note("dogfood report requires a sealed/redacted repro bundle ref")?;
        }
        Ok(())
    }

    fn push_note(&mut self, note: impl Into<String>) -> Result<()> {
        self.diagnostics
            .push_limited_value(note.into(), MAX_OPERATOR_DIAGNOSTICS, "dogfood report diagnostics")
    }
}

pub fn dogfood_report_value(input: &DogfoodReportInput<'_>) -> Result<IoValue> {
    let workflow = parse_operator_workflow(input.workflow_value)?;
    validate_refs(input.gate_receipt_refs, "dogfood gate receipt ref")?;
    validate_refs(input.repro_bundle_refs, "dogfood repro bundle ref")?;
    validate_ref(input.final_state_ref, "dogfood final state ref")?;
    let parts = ReportParts::collect(input, &workflow)?;
    let decision = if parts.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(crate::preserves_rail::record("dogfood-report-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("workflow", vec![crate::preserves_rail::string(&workflow.workflow_ref)]),
        crate::preserves_rail::record("checkpoints", vec![refs_sequence(&parts.checkpoint_refs)]),
        crate::preserves_rail::record("step-receipts", vec![step_receipts_sequence(&parts.step_receipts)]),
        crate::preserves_rail::record("gate-receipts", vec![refs_sequence(input.gate_receipt_refs)]),
        crate::preserves_rail::record("repro-bundles", vec![refs_sequence(input.repro_bundle_refs)]),
        crate::preserves_rail::record("final-state", vec![crate::preserves_rail::string(input.final_state_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&parts.diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-report", "pass"),
            (
                "deterministic-or-recorded",
                status(parts.diagnostics.iter().all(|item| !item.contains("replay status"))),
            ),
            ("final-state-bound", "pass"),
            ("redaction-gate", status(!input.repro_bundle_refs.is_empty())),
            ("no-text-oracle", "pass"),
            (
                "no-hidden-bypass",
                status(workflow.checks.iter().any(|(name, status)| name == "no-hidden-bypass" && status == "pass")),
            ),
        ]),
    ]))
}

pub fn parse_dogfood_report(value: &IoValue) -> Result<DogfoodReport> {
    let fields = value
        .collect_simple_record("dogfood-report-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dogfood-report-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA, "dogfood report")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-report", "dogfood report")?;
    require_check(&checks, "final-state-bound", "dogfood report")?;
    require_check(&checks, "no-text-oracle", "dogfood report")?;
    Ok(DogfoodReport {
        report_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        workflow_ref: record_ref(&fields[2], "workflow")?,
        checkpoint_refs: record_ref_sequence(&fields[3], "checkpoints")?,
        step_receipts: record_step_receipts(&fields[4], "step-receipts")?,
        gate_receipts: record_ref_sequence(&fields[5], "gate-receipts")?,
        repro_bundles: record_ref_sequence(&fields[6], "repro-bundles")?,
        final_state_ref: record_ref(&fields[7], "final-state")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_gate_receipt_value(input: &ReleaseGateInput<'_>) -> Result<IoValue> {
    let report = parse_dogfood_report(input.report_value)?;
    if report.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "dogfood release gate requires pass report {}; decision is {}",
            report.report_ref, report.decision
        )));
    }
    validate_ref(input.node_startup_ref, "dogfood release gate startup ref")?;
    validate_ref(input.node_shutdown_ref, "dogfood release gate shutdown ref")?;
    require_non_empty_refs(input.harness_gate_refs, "dogfood release harness gate ref")?;
    require_non_empty_refs(input.catalog_query_refs, "dogfood release catalog query ref")?;
    require_non_empty_refs(input.repro_verify_refs, "dogfood release repro verify ref")?;
    require_non_empty_refs(input.replay_index_refs, "dogfood release replay index ref")?;
    require_non_empty_refs(input.gc_refs, "dogfood release retention GC ref")?;
    require_non_empty_refs(input.validation_command_refs, "dogfood release validation command ref")?;
    Ok(crate::preserves_rail::record("release-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("node", vec![
            crate::preserves_rail::string(input.node_startup_ref),
            crate::preserves_rail::string(input.node_shutdown_ref),
        ]),
        crate::preserves_rail::record("harness-gates", vec![refs_sequence(input.harness_gate_refs)]),
        crate::preserves_rail::record("catalog-queries", vec![refs_sequence(input.catalog_query_refs)]),
        crate::preserves_rail::record("repro-verifies", vec![refs_sequence(input.repro_verify_refs)]),
        crate::preserves_rail::record("replay-indexes", vec![refs_sequence(input.replay_index_refs)]),
        crate::preserves_rail::record("retention-gc", vec![refs_sequence(input.gc_refs)]),
        crate::preserves_rail::record("validation-commands", vec![refs_sequence(input.validation_command_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("deterministic-or-recorded-only", "pass"),
            ("redaction-gate-bound", "pass"),
            ("startup-shutdown-bound", "pass"),
            ("catalog-mcp-bound", "pass"),
            ("replay-evidence-index-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("retention-gc-review-bound", "pass"),
            ("retention-gc-is-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_gate_receipt(value: &IoValue) -> Result<ReleaseGateReceipt> {
    let fields = value
        .collect_simple_record("release-gate-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA, "operator release gate")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "dogfood-report-pass", "operator release gate")?;
    require_check(&checks, "replay-evidence-index-bound", "operator release gate")?;
    require_check(&checks, "replay-index-is-evidence-only", "operator release gate")?;
    require_check(&checks, "no-text-oracle", "operator release gate")?;
    let node = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let node_fields = simple_record(&node, "node", 2)?;
    Ok(ReleaseGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        report_ref: record_ref(&fields[2], "report")?,
        startup_ref: required_ref(&node_fields[0], "release gate startup ref")?,
        shutdown_ref: required_ref(&node_fields[1], "release gate shutdown ref")?,
        harness_gate_refs: record_ref_sequence(&fields[4], "harness-gates")?,
        catalog_query_refs: record_ref_sequence(&fields[5], "catalog-queries")?,
        repro_verify_refs: record_ref_sequence(&fields[6], "repro-verifies")?,
        replay_index_refs: record_ref_sequence(&fields[7], "replay-indexes")?,
        gc_refs: record_ref_sequence(&fields[8], "retention-gc")?,
        validation_command_refs: record_ref_sequence(&fields[9], "validation-commands")?,
        checks,
        value: value.clone(),
    })
}

pub fn nix_dogfood_release_evidence_value(input: &NixDogfoodEvidenceInput<'_>) -> Result<IoValue> {
    let observed = observe_nix_dogfood_output(input.output_path)?;
    Ok(crate::preserves_rail::record("nix-dogfood-release-evidence-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(&observed.report_ref)]),
        crate::preserves_rail::record("release-gate", vec![crate::preserves_rail::string(&observed.release_gate_ref)]),
        crate::preserves_rail::record("replay-verify", vec![crate::preserves_rail::string(
            &observed.replay_verify_ref,
        )]),
        crate::preserves_rail::record("replay-index", vec![crate::preserves_rail::string(&observed.replay_index_ref)]),
        crate::preserves_rail::record("summary", vec![crate::preserves_rail::string(&observed.summary_ref)]),
        crate::preserves_rail::record("nextest", vec![
            crate::preserves_rail::string(&observed.nextest_marker_ref),
            crate::preserves_rail::string(observed.nextest_check_path.as_str()),
        ]),
        crate::preserves_rail::record("files", vec![file_refs_sequence(&observed.file_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-ref-bound", "pass"),
            ("replay-verify-ref-bound", "pass"),
            ("replay-index-ref-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-output-path-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_nix_dogfood_evidence(value: &IoValue) -> Result<NixDogfoodEvidence> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-evidence-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-evidence-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA, "Nix dogfood evidence")?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let nextest = crate::preserves_rail::value_to_iovalue(&fields[7]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "replay-verify-ref-bound", "Nix dogfood evidence")?;
    require_check(&checks, "replay-index-ref-bound", "Nix dogfood evidence")?;
    require_check(&checks, "replay-index-is-evidence-only", "Nix dogfood evidence")?;
    require_check(&checks, "release-evidence-only", "Nix dogfood evidence")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood evidence")?;
    Ok(NixDogfoodEvidence {
        evidence_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "Nix dogfood output path")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood output path ref")?,
        report_ref: record_ref(&fields[2], "report")?,
        release_gate_ref: record_ref(&fields[3], "release-gate")?,
        replay_verify_ref: record_ref(&fields[4], "replay-verify")?,
        replay_index_ref: record_ref(&fields[5], "replay-index")?,
        summary_ref: record_ref(&fields[6], "summary")?,
        nextest_marker_ref: required_ref(&nextest_fields[0], "Nix dogfood nextest marker ref")?,
        nextest_check_path: required_string(&nextest_fields[1], "Nix dogfood nextest check path")?,
        file_refs: record_file_refs(&fields[8], "files")?,
        checks,
        value: value.clone(),
    })
}

struct NixObservation {
    observed: ObservedNixDogfoodOutput,
    is_output_observed: bool,
}

fn fallback_nix_output(
    output_path: String,
    output_path_ref: String,
    evidence: &NixDogfoodEvidence,
) -> ObservedNixDogfoodOutput {
    ObservedNixDogfoodOutput {
        output_path,
        output_path_ref,
        report_ref: evidence.report_ref.clone(),
        release_gate_ref: evidence.release_gate_ref.clone(),
        replay_verify_ref: evidence.replay_verify_ref.clone(),
        replay_index_ref: evidence.replay_index_ref.clone(),
        summary_ref: evidence.summary_ref.clone(),
        nextest_marker_ref: evidence.nextest_marker_ref.clone(),
        nextest_check_path: evidence.nextest_check_path.clone(),
        file_refs: evidence.file_refs.clone(),
    }
}

fn observed_nix_or_fallback(
    output_path: &Path,
    evidence: &NixDogfoodEvidence,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<NixObservation> {
    let output_path_string = output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    match observe_nix_dogfood_output(output_path) {
        Ok(observed) => Ok(NixObservation {
            observed,
            is_output_observed: true,
        }),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("Nix dogfood output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
            Ok(NixObservation {
                observed: fallback_nix_output(output_path_string, fallback_output_path_ref, evidence),
                is_output_observed: false,
            })
        }
    }
}

pub fn verify_nix_dogfood_evidence(input: &NixDogfoodVerifyInput<'_>) -> Result<NixDogfoodVerifyReceipt> {
    let evidence = parse_nix_dogfood_evidence(input.evidence_value)?;
    let mut diagnostics = Vec::new();
    let NixObservation {
        observed,
        is_output_observed,
    } = observed_nix_or_fallback(input.output_path, &evidence, &mut diagnostics)?;
    for diagnostic in [
        mismatch_diagnostic("output-path-ref", &evidence.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("report-ref", &evidence.report_ref, &observed.report_ref),
        mismatch_diagnostic("release-gate-ref", &evidence.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("replay-verify-ref", &evidence.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("replay-index-ref", &evidence.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("summary-ref", &evidence.summary_ref, &observed.summary_ref),
        mismatch_diagnostic("nextest-marker-ref", &evidence.nextest_marker_ref, &observed.nextest_marker_ref),
        mismatch_diagnostic("nextest-check-path", &evidence.nextest_check_path, &observed.nextest_check_path),
    ]
    .into_iter()
    .flatten()
    {
        diagnostics.push_limited_value(diagnostic, MAX_OPERATOR_DIAGNOSTICS, "Nix dogfood verify diagnostics")?;
    }
    for diagnostic in file_ref_mismatch_diagnostics(&evidence.file_refs, &observed.file_refs)? {
        diagnostics.push_limited_value(diagnostic, MAX_OPERATOR_DIAGNOSTICS, "Nix dogfood verify diagnostics")?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("nix-dogfood-release-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::string(&evidence.evidence_ref)]),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(&observed.report_ref)]),
        crate::preserves_rail::record("release-gate", vec![crate::preserves_rail::string(&observed.release_gate_ref)]),
        crate::preserves_rail::record("replay-verify", vec![crate::preserves_rail::string(
            &observed.replay_verify_ref,
        )]),
        crate::preserves_rail::record("replay-index", vec![crate::preserves_rail::string(&observed.replay_index_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-ref-bound", status(evidence.release_gate_ref == observed.release_gate_ref)),
            ("replay-verify-ref-bound", status(evidence.replay_verify_ref == observed.replay_verify_ref)),
            ("replay-index-ref-bound", status(evidence.replay_index_ref == observed.replay_index_ref)),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-output-path-bound", status(evidence.output_path_ref == observed.output_path_ref)),
            ("nextest-dependency-bound", status(evidence.nextest_marker_ref == observed.nextest_marker_ref)),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]);
    parse_nix_dogfood_verify_receipt(&value)
}

pub fn parse_nix_dogfood_verify_receipt(value: &IoValue) -> Result<NixDogfoodVerifyReceipt> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-verify-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA,
        "Nix dogfood verify receipt",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "replay-verify-ref-bound", "Nix dogfood verify receipt")?;
    require_check(&checks, "replay-index-ref-bound", "Nix dogfood verify receipt")?;
    require_check(&checks, "replay-index-is-evidence-only", "Nix dogfood verify receipt")?;
    require_check(&checks, "release-evidence-only", "Nix dogfood verify receipt")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood verify receipt")?;
    Ok(NixDogfoodVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        evidence_ref: record_ref(&fields[2], "evidence")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood verify output path ref")?,
        report_ref: record_ref(&fields[4], "report")?,
        release_gate_ref: record_ref(&fields[5], "release-gate")?,
        replay_verify_ref: record_ref(&fields[6], "replay-verify")?,
        replay_index_ref: record_ref(&fields[7], "replay-index")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_evidence_bundle_value(input: &ReleaseEvidenceBundleInput<'_>) -> Result<IoValue> {
    let observed = observe_release_bundle_output(input.output_path)?;
    Ok(crate::preserves_rail::record("release-evidence-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("members", vec![file_refs_sequence(&observed.member_refs)]),
        crate::preserves_rail::record("dogfood", vec![
            crate::preserves_rail::string(&observed.report_ref),
            crate::preserves_rail::string(&observed.release_gate_ref),
        ]),
        crate::preserves_rail::record("replay", vec![
            crate::preserves_rail::string(&observed.replay_verify_ref),
            crate::preserves_rail::string(&observed.replay_index_ref),
        ]),
        crate::preserves_rail::record("nix", vec![
            crate::preserves_rail::string(&observed.nix_evidence_ref),
            crate::preserves_rail::string(&observed.nix_verify_ref),
        ]),
        crate::preserves_rail::record("nextest", vec![
            crate::preserves_rail::string(&observed.nextest_marker_ref),
            crate::preserves_rail::string(observed.nextest_check_path.as_str()),
        ]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-pass", "pass"),
            ("replay-verify-bound", "pass"),
            ("replay-index-bound", "pass"),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-verify-pass", "pass"),
            ("bundle-members-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_evidence_bundle(value: &IoValue) -> Result<ReleaseEvidenceBundle> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA,
        "release evidence bundle",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let replay = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let replay_fields = simple_record(&replay, "replay", 2)?;
    let nix = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let nextest = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle")?;
    require_check(&checks, "replay-verify-bound", "release evidence bundle")?;
    require_check(&checks, "replay-index-bound", "release evidence bundle")?;
    require_check(&checks, "replay-index-is-evidence-only", "release evidence bundle")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle")?;
    Ok(ReleaseEvidenceBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "release evidence output path")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence release gate ref")?,
        replay_verify_ref: required_ref(&replay_fields[0], "release evidence replay verify ref")?,
        replay_index_ref: required_ref(&replay_fields[1], "release evidence replay index ref")?,
        nix_evidence_ref: required_ref(&nix_fields[0], "release evidence Nix evidence ref")?,
        nix_verify_ref: required_ref(&nix_fields[1], "release evidence Nix verify ref")?,
        summary_ref: member_ref(&fields[2], "dogfood-summary.txt")?,
        nextest_marker_ref: required_ref(&nextest_fields[0], "release evidence nextest marker ref")?,
        nextest_check_path: required_string(&nextest_fields[1], "release evidence nextest check path")?,
        member_refs: record_file_refs(&fields[2], "members")?,
        checks,
        value: value.clone(),
    })
}

struct BundleObservation {
    observed: ObservedReleaseBundleOutput,
    is_output_observed: bool,
}

fn fallback_output(
    output_path: String,
    output_path_ref: String,
    bundle: &ReleaseEvidenceBundle,
) -> ObservedReleaseBundleOutput {
    ObservedReleaseBundleOutput {
        output_path,
        output_path_ref,
        report_ref: bundle.report_ref.clone(),
        release_gate_ref: bundle.release_gate_ref.clone(),
        replay_verify_ref: bundle.replay_verify_ref.clone(),
        replay_index_ref: bundle.replay_index_ref.clone(),
        nix_evidence_ref: bundle.nix_evidence_ref.clone(),
        nix_verify_ref: bundle.nix_verify_ref.clone(),
        summary_ref: bundle.summary_ref.clone(),
        nextest_marker_ref: bundle.nextest_marker_ref.clone(),
        nextest_check_path: bundle.nextest_check_path.clone(),
        member_refs: bundle.member_refs.clone(),
    }
}

fn observed_or_fallback(
    output_path: &Path,
    bundle: &ReleaseEvidenceBundle,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<BundleObservation> {
    let output_path_string = output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    match observe_release_bundle_output(output_path) {
        Ok(observed) => Ok(BundleObservation {
            observed,
            is_output_observed: true,
        }),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("release evidence bundle output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release evidence bundle verify diagnostics",
            )?;
            Ok(BundleObservation {
                observed: fallback_output(output_path_string, fallback_output_path_ref, bundle),
                is_output_observed: false,
            })
        }
    }
}

pub fn verify_release_evidence_bundle(
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let bundle = parse_release_evidence_bundle(input.bundle_value)?;
    let mut diagnostics = Vec::new();
    let BundleObservation {
        observed,
        is_output_observed,
    } = observed_or_fallback(input.output_path, &bundle, &mut diagnostics)?;
    for diagnostic in release_bundle_mismatch_diagnostics(&bundle, &observed)? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    let signature_diagnostics = release_bundle_signature_diagnostics(&bundle, input)?;
    let is_signed_member_receipts_ok = signature_diagnostics.is_empty();
    for diagnostic in signature_diagnostics {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-evidence-bundle-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&bundle.bundle_ref)]),
        crate::preserves_rail::record("output-path", vec![
            crate::preserves_rail::string(observed.output_path.as_str()),
            crate::preserves_rail::string(&observed.output_path_ref),
        ]),
        crate::preserves_rail::record("dogfood", vec![
            crate::preserves_rail::string(&observed.report_ref),
            crate::preserves_rail::string(&observed.release_gate_ref),
        ]),
        crate::preserves_rail::record("replay", vec![
            crate::preserves_rail::string(&observed.replay_verify_ref),
            crate::preserves_rail::string(&observed.replay_index_ref),
        ]),
        crate::preserves_rail::record("nix", vec![
            crate::preserves_rail::string(&observed.nix_evidence_ref),
            crate::preserves_rail::string(&observed.nix_verify_ref),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-pass", status(is_output_observed)),
            ("replay-verify-bound", status(is_output_observed)),
            ("replay-index-bound", status(is_output_observed)),
            ("replay-index-is-evidence-only", "pass"),
            ("nix-verify-pass", status(is_output_observed)),
            ("bundle-members-bound", status(diagnostics.is_empty())),
            ("signed-member-receipts", status(is_signed_member_receipts_ok)),
            ("signed-receipts-evidence-only", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]);
    parse_release_evidence_bundle_verify_receipt(&value)
}

pub fn parse_release_evidence_bundle_verify_receipt(value: &IoValue) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-verify-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "release evidence bundle verify receipt",
    )?;
    let output_path = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let replay = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let replay_fields = simple_record(&replay, "replay", 2)?;
    let nix = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-verify-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-index-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "replay-index-is-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-member-receipts", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-receipts-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle verify receipt")?;
    Ok(ReleaseEvidenceBundleVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        bundle_ref: record_ref(&fields[2], "bundle")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence verify output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence verify report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence verify release gate ref")?,
        replay_verify_ref: required_ref(&replay_fields[0], "release evidence verify replay verify ref")?,
        replay_index_ref: required_ref(&replay_fields[1], "release evidence verify replay index ref")?,
        nix_evidence_ref: required_ref(&nix_fields[0], "release evidence verify Nix evidence ref")?,
        nix_verify_ref: required_ref(&nix_fields[1], "release evidence verify Nix verify ref")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

struct PromotionFacts {
    output_path_ref: String,
    source_ref: String,
    octet_ref: String,
    cairn_ref: String,
    diagnostics: Vec<String>,
    key: PromotionKeyFacts,
    key_revocation_refs: Vec<String>,
}

struct PromotionKeyFacts {
    selected_key_ref: String,
    selected_key_id: String,
    selected_signer: String,
    selected_trust_root: String,
    selected_generation: u64,
    has_selected_key: bool,
    diagnostic: Option<String>,
}

pub fn release_promotion_gate_receipt_value(
    input: &ReleasePromotionGateInput<'_>,
) -> Result<ReleasePromotionGateReceipt> {
    let bundle_verify = parse_release_evidence_bundle_verify_receipt(input.bundle_verify_value)?;
    let facts = promotion_facts(input, &bundle_verify)?;
    let decision = if facts.diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-promotion-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("bundle-verify", vec![
            crate::preserves_rail::string(&bundle_verify.receipt_ref),
            crate::preserves_rail::string(&bundle_verify.bundle_ref),
            crate::preserves_rail::string(&bundle_verify.output_path_ref),
            crate::preserves_rail::string(&bundle_verify.report_ref),
            crate::preserves_rail::string(&bundle_verify.release_gate_ref),
            crate::preserves_rail::string(&bundle_verify.nix_evidence_ref),
            crate::preserves_rail::string(&bundle_verify.nix_verify_ref),
        ]),
        crate::preserves_rail::record("signed-keyring", vec![
            crate::preserves_rail::record("selected-key", vec![
                crate::preserves_rail::string(&facts.key.selected_key_ref),
                crate::preserves_rail::string(&facts.key.selected_key_id),
                crate::preserves_rail::string(&facts.key.selected_signer),
                crate::preserves_rail::string(&facts.key.selected_trust_root),
                crate::preserves_rail::u64_value(facts.key.selected_generation),
            ]),
            refs_sequence(&facts.key_revocation_refs),
        ]),
        crate::preserves_rail::record("evidence", vec![
            crate::preserves_rail::record("source", vec![
                crate::preserves_rail::string(input.source_evidence),
                crate::preserves_rail::string(&facts.source_ref),
            ]),
            crate::preserves_rail::record("octet", vec![
                crate::preserves_rail::string(input.octet_evidence),
                crate::preserves_rail::string(&facts.octet_ref),
            ]),
            crate::preserves_rail::record("cairn", vec![
                crate::preserves_rail::string(input.cairn_evidence),
                crate::preserves_rail::string(&facts.cairn_ref),
            ]),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&facts.diagnostics)]),
        checks_value_from_pairs(&[
            ("release-bundle-verify-pass", status(bundle_verify.decision == "pass")),
            ("promotion-output-path-bound", status(facts.output_path_ref == bundle_verify.output_path_ref)),
            ("signed-keyring-current", status(facts.key.has_selected_key)),
            ("source-evidence-bound", status(!input.source_evidence.trim().is_empty())),
            ("octet-evidence-bound", status(!input.octet_evidence.trim().is_empty())),
            ("cairn-evidence-bound", status(!input.cairn_evidence.trim().is_empty())),
            ("release-promotion-is-evidence-only", "pass"),
            ("no-subsystem-authority-granted", "pass"),
        ]),
    ]);
    parse_release_promotion_gate_receipt(&value)
}

fn promotion_facts(
    input: &ReleasePromotionGateInput<'_>,
    bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
) -> Result<PromotionFacts> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let source_ref = raw_text_ref("molten.operator.release-promotion.source-evidence.v1", input.source_evidence);
    let octet_ref = raw_text_ref("molten.operator.release-promotion.octet-evidence.v1", input.octet_evidence);
    let cairn_ref = raw_text_ref("molten.operator.release-promotion.cairn-evidence.v1", input.cairn_evidence);
    let key = promotion_key_facts(input)?;
    let mut diagnostics = promotion_diagnostics(input, bundle_verify, &output_path_ref)?;
    if let Some(diagnostic) = key.diagnostic.as_ref() {
        diagnostics.push_limited_value(
            diagnostic.clone(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    let key_revocation_refs = input
        .signed_key_revocations
        .iter()
        .map(|revocation| revocation.revocation_ref.clone())
        .collect::<Vec<_>>();
    Ok(PromotionFacts {
        output_path_ref,
        source_ref,
        octet_ref,
        cairn_ref,
        diagnostics,
        key,
        key_revocation_refs,
    })
}

fn promotion_diagnostics(
    input: &ReleasePromotionGateInput<'_>,
    bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
    output_path_ref: &str,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if bundle_verify.decision != "pass" {
        diagnostics.push_limited_value(
            format!(
                "release evidence bundle verify receipt {} decision is {}",
                bundle_verify.receipt_ref, bundle_verify.decision
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if output_path_ref != bundle_verify.output_path_ref {
        diagnostics.push_limited_value(
            format!(
                "promotion output-path-ref mismatch: receipt={} observed={}",
                bundle_verify.output_path_ref, output_path_ref
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.source_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "source evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.octet_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "Octet evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    if input.cairn_evidence.trim().is_empty() {
        diagnostics.push_limited_value(
            "Cairn evidence marker must not be empty".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn promotion_key_facts(input: &ReleasePromotionGateInput<'_>) -> Result<PromotionKeyFacts> {
    match select_release_promotion_key(input) {
        Ok(key) => Ok(PromotionKeyFacts {
            selected_key_ref: key.key_ref.clone(),
            selected_key_id: key.key_id.clone(),
            selected_signer: key.signer.clone(),
            selected_trust_root: key.trust_root.clone(),
            selected_generation: key.generation,
            has_selected_key: true,
            diagnostic: None,
        }),
        Err(error) => Ok(PromotionKeyFacts {
            selected_key_ref: dogfood_ref("missing-signed-key")?,
            selected_key_id: "missing".to_string(),
            selected_signer: input.signed_signer.unwrap_or("missing").to_string(),
            selected_trust_root: input.signed_trust_root.to_string(),
            selected_generation: 0,
            has_selected_key: false,
            diagnostic: Some(format!("signed keyring currentness failed: {error}")),
        }),
    }
}

pub fn parse_release_promotion_gate_receipt(value: &IoValue) -> Result<ReleasePromotionGateReceipt> {
    let fields = value
        .collect_simple_record("release-promotion-gate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-gate-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA,
        "release promotion gate receipt",
    )?;
    let bundle_value = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let bundle_fields = simple_record(&bundle_value, "bundle-verify", 7)?;
    let keyring_value = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let keyring_fields = simple_record(&keyring_value, "signed-keyring", 2)?;
    let selected_key_value = crate::preserves_rail::value_to_iovalue(&keyring_fields[0]);
    let selected_key_fields = simple_record(&selected_key_value, "selected-key", 5)?;
    let evidence_value = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 2)?;
    let octet_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 2)?;
    let cairn_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[2]);
    let cairn_fields = simple_record(&cairn_value, "cairn", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "release-bundle-verify-pass", "release promotion gate receipt")?;
    require_check(&checks, "promotion-output-path-bound", "release promotion gate receipt")?;
    require_check(&checks, "signed-keyring-current", "release promotion gate receipt")?;
    require_check(&checks, "source-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "octet-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "cairn-evidence-bound", "release promotion gate receipt")?;
    require_check(&checks, "release-promotion-is-evidence-only", "release promotion gate receipt")?;
    require_check(&checks, "no-subsystem-authority-granted", "release promotion gate receipt")?;
    Ok(ReleasePromotionGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        bundle_verify_ref: required_ref(&bundle_fields[0], "release promotion bundle verify receipt ref")?,
        bundle_ref: required_ref(&bundle_fields[1], "release promotion bundle ref")?,
        output_path_ref: required_ref(&bundle_fields[2], "release promotion output path ref")?,
        selected_key_ref: required_ref(&selected_key_fields[0], "release promotion signed key ref")?,
        source_ref: required_ref(&source_fields[1], "release promotion source evidence ref")?,
        octet_ref: required_ref(&octet_fields[1], "release promotion Octet evidence ref")?,
        cairn_ref: required_ref(&cairn_fields[1], "release promotion Cairn evidence ref")?,
        diagnostics: record_string_sequence(&fields[5], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

struct GateReadback {
    promotion: Option<ReleasePromotionGateReceipt>,
    diagnostics: Vec<String>,
}

struct SummarySigned {
    envelope_ref: String,
    subject_ref: String,
    key_ref: String,
}

struct SignedReadback {
    signed: Option<SummarySigned>,
    diagnostics: Vec<String>,
}

struct SummaryFacts {
    promotion: Option<ReleasePromotionGateReceipt>,
    signed: Option<SummarySigned>,
    diagnostics: Vec<String>,
}

struct SummaryRefs {
    promotion_ref: String,
    promotion_decision: String,
    bundle_verify_ref: String,
    bundle_ref: String,
    source_ref: String,
    octet_ref: String,
    cairn_ref: String,
    signed_envelope_ref: String,
    signed_subject_ref: String,
    signed_key_ref: String,
}

pub fn release_promotion_summary_value(input: &ReleasePromotionSummaryInput<'_>) -> Result<ReleasePromotionSummary> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let facts = summary_facts(input, &output_path_ref)?;
    let refs = summary_refs(&facts)?;
    let value = summary_record(&output_path_string, &output_path_ref, &facts, &refs);
    parse_release_promotion_summary(&value)
}

fn summary_facts(input: &ReleasePromotionSummaryInput<'_>, output_path_ref: &str) -> Result<SummaryFacts> {
    let gate = read_summary_gate(input, output_path_ref)?;
    let expected_subject_ref = gate.promotion.as_ref().map(|promotion| promotion.receipt_ref.as_str());
    let signed = read_signed_summary(input, expected_subject_ref)?;
    let mut diagnostics = gate.diagnostics;
    for diagnostic in signed.diagnostics {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    Ok(SummaryFacts {
        promotion: gate.promotion,
        signed: signed.signed,
        diagnostics,
    })
}

fn read_summary_gate(input: &ReleasePromotionSummaryInput<'_>, output_path_ref: &str) -> Result<GateReadback> {
    let mut diagnostics = Vec::new();
    let promotion_result = read_output_text(input.output_path, "release-promotion-gate.preserves")
        .and_then(|text| crate::preserves_rail::parse_text(&text))
        .and_then(|value| parse_release_promotion_gate_receipt(&value));
    let promotion = match promotion_result {
        Ok(promotion) => Some(promotion),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("release promotion gate receipt readback failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion summary diagnostics",
            )?;
            None
        }
    };
    if let Some(promotion) = promotion.as_ref() {
        for diagnostic in summary_gate_diagnostics(promotion, output_path_ref)? {
            diagnostics.push_limited_value(
                diagnostic,
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion summary diagnostics",
            )?;
        }
    }
    Ok(GateReadback { promotion, diagnostics })
}

fn summary_gate_diagnostics(promotion: &ReleasePromotionGateReceipt, output_path_ref: &str) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if promotion.decision != "pass" {
        diagnostics.push_limited_value(
            format!("release promotion gate receipt {} decision is {}", promotion.receipt_ref, promotion.decision),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    if promotion.output_path_ref != output_path_ref {
        diagnostics.push_limited_value(
            format!(
                "release promotion summary output-path-ref mismatch: receipt={} observed={}",
                promotion.output_path_ref, output_path_ref
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "release promotion summary diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn read_signed_summary(
    input: &ReleasePromotionSummaryInput<'_>,
    expected_subject_ref: Option<&str>,
) -> Result<SignedReadback> {
    let signed_result = read_output_text(input.output_path, "release-promotion-gate.signed.preserves")
        .and_then(|text| crate::preserves_rail::parse_text(&text))
        .and_then(|value| {
            verify_signed_receipt_with_keyring_policy(&value, &VerifySignedReceiptKeyringPolicy {
                required_purpose: RELEASE_PROMOTION_SIGNING_PURPOSE,
                trust_root: input.signed_trust_root,
                expected_signer: input.signed_signer,
                expected_subject_ref,
                required_key_ref: input.signed_key_ref,
                required_key_id: input.signed_key_id,
                keys: input.signed_keys,
                revocations: input.signed_key_revocations,
            })
        });
    match signed_result {
        Ok(signed) => Ok(SignedReadback {
            signed: Some(SummarySigned {
                envelope_ref: signed.receipt.envelope_ref,
                subject_ref: signed.receipt.subject_ref,
                key_ref: signed.key_ref,
            }),
            diagnostics: Vec::new(),
        }),
        Err(error) => Ok(SignedReadback {
            signed: None,
            diagnostics: vec![format!("signed promotion receipt verification failed: {error}")],
        }),
    }
}

fn summary_refs(facts: &SummaryFacts) -> Result<SummaryRefs> {
    let promotion_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-promotion-gate"), |promotion| Ok(promotion.receipt_ref.clone()))?;
    let promotion_decision = facts
        .promotion
        .as_ref()
        .map_or_else(|| "missing".to_string(), |promotion| promotion.decision.clone());
    let bundle_verify_ref = facts.promotion.as_ref().map_or_else(
        || dogfood_ref("missing-release-bundle-verify"),
        |promotion| Ok(promotion.bundle_verify_ref.clone()),
    )?;
    let bundle_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-evidence-bundle"), |promotion| Ok(promotion.bundle_ref.clone()))?;
    let source_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-source-evidence"), |promotion| Ok(promotion.source_ref.clone()))?;
    let octet_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-octet-evidence"), |promotion| Ok(promotion.octet_ref.clone()))?;
    let cairn_ref = facts
        .promotion
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-cairn-evidence"), |promotion| Ok(promotion.cairn_ref.clone()))?;
    let signed_envelope_ref = facts
        .signed
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-signed-release-promotion"), |signed| Ok(signed.envelope_ref.clone()))?;
    let signed_subject_ref = facts.signed.as_ref().map_or_else(
        || dogfood_ref("missing-signed-release-promotion-subject"),
        |signed| Ok(signed.subject_ref.clone()),
    )?;
    let signed_key_ref = facts
        .signed
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-signed-release-key"), |signed| Ok(signed.key_ref.clone()))?;
    Ok(SummaryRefs {
        promotion_ref,
        promotion_decision,
        bundle_verify_ref,
        bundle_ref,
        source_ref,
        octet_ref,
        cairn_ref,
        signed_envelope_ref,
        signed_subject_ref,
        signed_key_ref,
    })
}

fn summary_record(
    output_path_string: &str,
    output_path_ref: &str,
    facts: &SummaryFacts,
    refs: &SummaryRefs,
) -> IoValue {
    let decision = if facts.diagnostics.is_empty() { "pass" } else { "deny" };
    crate::preserves_rail::record("release-promotion-summary-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("output", vec![
            crate::preserves_rail::string(output_path_string),
            crate::preserves_rail::string(output_path_ref),
        ]),
        crate::preserves_rail::record("promotion", vec![
            crate::preserves_rail::string(&refs.promotion_ref),
            crate::preserves_rail::string(&refs.promotion_decision),
            crate::preserves_rail::string(&refs.bundle_verify_ref),
            crate::preserves_rail::string(&refs.bundle_ref),
        ]),
        crate::preserves_rail::record("signed-promotion", vec![
            crate::preserves_rail::string(&refs.signed_envelope_ref),
            crate::preserves_rail::string(&refs.signed_subject_ref),
            crate::preserves_rail::string(&refs.signed_key_ref),
            crate::preserves_rail::string(RELEASE_PROMOTION_SIGNING_PURPOSE),
        ]),
        crate::preserves_rail::record("evidence", vec![
            crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&refs.source_ref)]),
            crate::preserves_rail::record("octet", vec![crate::preserves_rail::string(&refs.octet_ref)]),
            crate::preserves_rail::record("cairn", vec![crate::preserves_rail::string(&refs.cairn_ref)]),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&facts.diagnostics)]),
        checks_value_from_pairs(&[
            (
                "release-promotion-pass",
                status(facts.promotion.as_ref().is_some_and(|promotion| promotion.decision == "pass")),
            ),
            (
                "release-promotion-output-bound",
                status(facts.promotion.as_ref().is_some_and(|promotion| promotion.output_path_ref == output_path_ref)),
            ),
            ("signed-promotion-present", status(facts.signed.is_some())),
            (
                "signed-promotion-subject-bound",
                status(facts.signed.as_ref().is_some_and(|signed| signed.subject_ref == refs.promotion_ref)),
            ),
            ("signed-promotion-keyring-current", status(facts.signed.is_some())),
            ("release-promotion-summary-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ])
}

pub fn parse_release_promotion_summary(value: &IoValue) -> Result<ReleasePromotionSummary> {
    let fields = value
        .collect_simple_record("release-promotion-summary-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-summary-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA,
        "release promotion summary",
    )?;
    let promotion_value = crate::preserves_rail::value_to_iovalue(&fields[3]);
    let promotion_fields = simple_record(&promotion_value, "promotion", 4)?;
    let signed_value = crate::preserves_rail::value_to_iovalue(&fields[4]);
    let signed_fields = simple_record(&signed_value, "signed-promotion", 4)?;
    let evidence_value = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 1)?;
    let octet_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 1)?;
    let cairn_value = crate::preserves_rail::value_to_iovalue(&evidence_fields[2]);
    let cairn_fields = simple_record(&cairn_value, "cairn", 1)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "release-promotion-pass", "release promotion summary")?;
    require_check(&checks, "release-promotion-output-bound", "release promotion summary")?;
    require_check(&checks, "signed-promotion-present", "release promotion summary")?;
    require_check(&checks, "signed-promotion-subject-bound", "release promotion summary")?;
    require_check(&checks, "signed-promotion-keyring-current", "release promotion summary")?;
    require_check(&checks, "release-promotion-summary-is-evidence-only", "release promotion summary")?;
    require_check(&checks, "no-release-authority-granted", "release promotion summary")?;
    Ok(ReleasePromotionSummary {
        summary_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        promotion_ref: required_ref(&promotion_fields[0], "release promotion summary promotion ref")?,
        bundle_verify_ref: required_ref(&promotion_fields[2], "release promotion summary bundle verify ref")?,
        signed_envelope_ref: required_ref(&signed_fields[0], "release promotion summary signed envelope ref")?,
        signed_subject_ref: required_ref(&signed_fields[1], "release promotion summary signed subject ref")?,
        signed_key_ref: required_ref(&signed_fields[2], "release promotion summary signed key ref")?,
        source_ref: required_ref(&source_fields[0], "release promotion summary source ref")?,
        octet_ref: required_ref(&octet_fields[0], "release promotion summary Octet ref")?,
        cairn_ref: required_ref(&cairn_fields[0], "release promotion summary Cairn ref")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_export_manifest_value(input: &ReleaseExportManifestInput<'_>) -> Result<ReleaseExportManifest> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let summary_value = crate::preserves_rail::parse_text(&read_output_text(
        input.output_path,
        "release-promotion-summary.preserves",
    )?)?;
    let summary = parse_release_promotion_summary(&summary_value)?;
    if summary.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release export requires pass promotion summary {}; decision is {}",
            summary.summary_ref, summary.decision
        )));
    }
    let member_refs = observe_release_export_members(input.output_path)?;
    let value = crate::preserves_rail::record("release-export-manifest-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA),
        crate::preserves_rail::record("output", vec![
            crate::preserves_rail::string(&output_path_string),
            crate::preserves_rail::string(&output_path_ref),
        ]),
        crate::preserves_rail::record("promotion-summary", vec![crate::preserves_rail::string(&summary.summary_ref)]),
        crate::preserves_rail::record("members", vec![file_refs_sequence(&member_refs)]),
        checks_value_from_pairs(&[
            ("release-promotion-summary-pass", "pass"),
            ("release-export-members-bound", "pass"),
            ("deterministic-archive-layout", "pass"),
            ("release-export-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_export_manifest(&value)
}

pub fn parse_release_export_manifest(value: &IoValue) -> Result<ReleaseExportManifest> {
    let fields = value
        .collect_simple_record("release-export-manifest-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-manifest-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA,
        "release export manifest",
    )?;
    let output_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_value, "output", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-promotion-summary-pass", "release export manifest")?;
    require_check(&checks, "release-export-members-bound", "release export manifest")?;
    require_check(&checks, "deterministic-archive-layout", "release export manifest")?;
    require_check(&checks, "release-export-is-evidence-only", "release export manifest")?;
    require_check(&checks, "no-release-authority-granted", "release export manifest")?;
    Ok(ReleaseExportManifest {
        manifest_ref: crate::preserves_rail::canonical_hash(value)?,
        output_path_ref: required_ref(&output_fields[1], "release export output path ref")?,
        promotion_summary_ref: record_ref(&fields[2], "promotion-summary")?,
        member_refs: record_file_refs(&fields[3], "members")?,
        checks,
        value: value.clone(),
    })
}

pub fn verify_release_export(input: &ReleaseExportVerifyInput<'_>) -> Result<ReleaseExportVerifyReceipt> {
    let mut diagnostics = input.archive_diagnostics.to_vec();
    let parsed_manifest = match input.manifest_value {
        Some(value) => Some(parse_release_export_manifest(value)?),
        None => {
            diagnostics.push_limited_value(
                "release export archive is missing manifest".to_string(),
                MAX_OPERATOR_DIAGNOSTICS,
                "release export verify diagnostics",
            )?;
            None
        }
    };
    if let Some(manifest) = parsed_manifest.as_ref() {
        for diagnostic in file_ref_mismatch_diagnostics(&manifest.member_refs, input.member_refs)? {
            diagnostics.push_limited_value(
                diagnostic,
                MAX_OPERATOR_DIAGNOSTICS,
                "release export verify diagnostics",
            )?;
        }
    }
    if input.member_refs.iter().any(|(name, _)| name == "release-export-manifest.preserves") {
        diagnostics.push_limited_value(
            "release export archive must not list manifest as a payload member".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release export verify diagnostics",
        )?;
    }
    let manifest_ref = parsed_manifest
        .as_ref()
        .map_or_else(|| dogfood_ref("missing-release-export-manifest"), |manifest| Ok(manifest.manifest_ref.clone()))?;
    let promotion_summary_ref = parsed_manifest.as_ref().map_or_else(
        || dogfood_ref("missing-release-promotion-summary"),
        |manifest| Ok(manifest.promotion_summary_ref.clone()),
    )?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = crate::preserves_rail::record("release-export-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("manifest", vec![
            crate::preserves_rail::string(&manifest_ref),
            crate::preserves_rail::string(&promotion_summary_ref),
        ]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("release-export-members-bound", status(diagnostics.is_empty())),
            ("release-promotion-summary-bound", status(parsed_manifest.is_some() && diagnostics.is_empty())),
            ("release-export-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_export_verify_receipt(&value)
}

pub fn parse_release_export_verify_receipt(value: &IoValue) -> Result<ReleaseExportVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-export-verify-receipt-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA,
        "release export verify receipt",
    )?;
    let manifest_value = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let manifest_fields = simple_record(&manifest_value, "manifest", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-export-members-bound", "release export verify receipt")?;
    require_check(&checks, "release-promotion-summary-bound", "release export verify receipt")?;
    require_check(&checks, "release-export-is-evidence-only", "release export verify receipt")?;
    require_check(&checks, "no-release-authority-granted", "release export verify receipt")?;
    Ok(ReleaseExportVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        manifest_ref: required_ref(&manifest_fields[0], "release export manifest ref")?,
        promotion_summary_ref: required_ref(&manifest_fields[1], "release export promotion summary ref")?,
        diagnostics: record_string_sequence(&fields[3], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

fn select_release_promotion_key<'a>(input: &'a ReleasePromotionGateInput<'_>) -> Result<&'a SignedReceiptKey> {
    let mut matches = Vec::new();
    for key in input.signed_keys {
        if key.trust_root != input.signed_trust_root {
            continue;
        }
        if let Some(signer) = input.signed_signer
            && key.signer != signer
        {
            continue;
        }
        if let Some(key_ref) = input.signed_key_ref
            && key.key_ref != key_ref
        {
            continue;
        }
        if let Some(key_id) = input.signed_key_id
            && key.key_id != key_id
        {
            continue;
        }
        matches.push_limited_value(key, MAX_OPERATOR_REFS, "release promotion signed key matches")?;
    }
    if matches.is_empty() {
        return Err(MoltenError::invalid_harness("no signed receipt key matched promotion policy"));
    }
    let mut current = Vec::new();
    for key in matches {
        if key.status != crate::evidence::SIGNED_RECEIPT_KEY_STATUS_CURRENT {
            continue;
        }
        if input.signed_key_revocations.iter().any(|revocation| revocation.key_ref == key.key_ref) {
            continue;
        }
        current.push_limited_value(key, MAX_OPERATOR_REFS, "release promotion current signed keys")?;
    }
    if current.is_empty() {
        Err(MoltenError::invalid_harness("matching signed receipt keys are stale or revoked"))
    } else if current.len() > 1 {
        Err(MoltenError::invalid_harness(
            "multiple current signed receipt keys matched promotion policy; specify key ref or key id",
        ))
    } else {
        Ok(current[0])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedNixDogfoodOutput {
    output_path: String,
    output_path_ref: String,
    report_ref: String,
    release_gate_ref: String,
    replay_verify_ref: String,
    replay_index_ref: String,
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    file_refs: Vec<(String, String)>,
}

struct OutputBindingRefs<'a> {
    report: &'a DogfoodReport,
    release_gate: &'a ReleaseGateReceipt,
    replay_verify_ref: &'a str,
    replay_index_ref: &'a str,
    replay_index_receipt_refs: &'a [String],
}

fn require_observed_bindings(input: &OutputBindingRefs<'_>) -> Result<()> {
    if !input
        .replay_index_receipt_refs
        .iter()
        .any(|reference| reference.as_str() == input.replay_verify_ref)
    {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood replay index {} does not bind replay verify {}",
            input.replay_index_ref, input.replay_verify_ref
        )));
    }
    if input.report.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass report {}; decision is {}",
            input.report.report_ref, input.report.decision
        )));
    }
    if input.release_gate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass release gate {}; decision is {}",
            input.release_gate.receipt_ref, input.release_gate.decision
        )));
    }
    if input.release_gate.report_ref != input.report.report_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood release gate report ref {} does not match report {}",
            input.release_gate.report_ref, input.report.report_ref
        )));
    }
    if !input
        .release_gate
        .replay_index_refs
        .iter()
        .any(|reference| reference.as_str() == input.replay_index_ref)
    {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood release gate does not bind replay index {}",
            input.replay_index_ref
        )));
    }
    Ok(())
}

fn observed_file_refs(entries: [(&str, &str); 6]) -> Result<Vec<(String, String)>> {
    let mut file_refs = Vec::new();
    for (path, reference) in entries {
        file_refs.push_limited_value(
            (path.to_string(), reference.to_string()),
            MAX_OPERATOR_REFS,
            "Nix dogfood file refs",
        )?;
    }
    Ok(file_refs)
}

fn observe_nix_dogfood_output(output_path: &Path) -> Result<ObservedNixDogfoodOutput> {
    let output_path_string = output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let report_text = read_output_text(output_path, "dogfood-report.preserves")?;
    let release_gate_text = read_output_text(output_path, "release-gate.preserves")?;
    let replay_verify_text = read_output_text(output_path, "replay-verify.preserves")?;
    let replay_index_text = read_output_text(output_path, "replay-evidence-index.preserves")?;
    let summary_text = read_output_text(output_path, "dogfood-summary.txt")?;
    let nextest_text = read_output_text(output_path, "after-nextest.txt")?;
    let report_value = crate::preserves_rail::parse_text(&report_text)?;
    let release_gate_value = crate::preserves_rail::parse_text(&release_gate_text)?;
    let replay_verify_value = crate::preserves_rail::parse_text(&replay_verify_text)?;
    let replay_index_value = crate::preserves_rail::parse_text(&replay_index_text)?;
    let report = parse_dogfood_report(&report_value)?;
    let release_gate = parse_release_gate_receipt(&release_gate_value)?;
    let replay_verify_ref = parse_release_replay_verify(&replay_verify_value)?;
    let replay_index_ref = parse_release_replay_index(&replay_index_value)?;
    let replay_index_receipt_refs = parse_release_replay_index_receipt_refs(&replay_index_value)?;
    require_observed_bindings(&OutputBindingRefs {
        report: &report,
        release_gate: &release_gate,
        replay_verify_ref: &replay_verify_ref,
        replay_index_ref: &replay_index_ref,
        replay_index_receipt_refs: &replay_index_receipt_refs,
    })?;
    let nextest_check_path = nextest_text.trim().to_string();
    if nextest_check_path.is_empty() {
        return Err(MoltenError::invalid_harness("Nix dogfood after-nextest marker is empty"));
    }
    let summary_ref = raw_text_ref("molten.operator.nix-dogfood-summary.v1", &summary_text);
    let nextest_marker_ref = raw_text_ref("molten.operator.nix-dogfood-nextest-marker.v1", &nextest_text);
    let file_refs = observed_file_refs([
        ("dogfood-report.preserves", report.report_ref.as_str()),
        ("release-gate.preserves", release_gate.receipt_ref.as_str()),
        ("replay-verify.preserves", replay_verify_ref.as_str()),
        ("replay-evidence-index.preserves", replay_index_ref.as_str()),
        ("dogfood-summary.txt", summary_ref.as_str()),
        ("after-nextest.txt", nextest_marker_ref.as_str()),
    ])?;
    Ok(ObservedNixDogfoodOutput {
        output_path: output_path_string,
        output_path_ref,
        report_ref: report.report_ref,
        release_gate_ref: release_gate.receipt_ref,
        replay_verify_ref,
        replay_index_ref,
        summary_ref,
        nextest_marker_ref,
        nextest_check_path,
        file_refs,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedReleaseBundleOutput {
    output_path: String,
    output_path_ref: String,
    report_ref: String,
    release_gate_ref: String,
    replay_verify_ref: String,
    replay_index_ref: String,
    nix_evidence_ref: String,
    nix_verify_ref: String,
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    member_refs: Vec<(String, String)>,
}

fn parse_release_replay_verify(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("deterministic-replay-verify-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-verify-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "release replay verify")?;
    let decision = required_string(&fields[1], "release replay verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay verify decision is {decision}; expected pass"
        )));
    }
    let divergence = record_string(&fields[10], "divergence")?;
    if divergence != "none" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay verify divergence is {divergence}; expected none"
        )));
    }
    crate::preserves_rail::canonical_hash(value)
}

fn parse_release_replay_index(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("deterministic-replay-index-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-index-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA, "release replay index")?;
    let decision = record_string(&fields[1], "decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release replay index decision is {decision}; expected pass"
        )));
    }
    let checks = parse_replay_index_checks(&fields[14])?;
    require_check(&checks, "evidence-only", "release replay index")?;
    require_check(&checks, "no-authority-grant", "release replay index")?;
    crate::preserves_rail::canonical_hash(value)
}

fn parse_release_replay_index_receipt_refs(value: &IoValue) -> Result<Vec<String>> {
    let fields = value
        .collect_simple_record("deterministic-replay-index-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <deterministic-replay-index-v1 ...>"))?;
    record_ref_sequence(&fields[7], "receipt-refs")
}

fn parse_replay_index_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let items = required_sequence(value, "release replay index checks")?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, "release replay index checks")?;
    let mut checks = Vec::new();
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let fields = simple_record(&item, "check", 2)?;
        let name = required_string(&fields[0], "release replay index check name")?;
        let status = required_string(&fields[1], "release replay index check status")?;
        checks.push_limited_value((name, status), MAX_OPERATOR_REFS, "release replay index checks")?;
    }
    Ok(checks)
}

fn observe_release_bundle_output(output_path: &Path) -> Result<ObservedReleaseBundleOutput> {
    let observed_nix = observe_nix_dogfood_output(output_path)?;
    let nix_evidence_value =
        crate::preserves_rail::parse_text(&read_output_text(output_path, "nix-dogfood-evidence.preserves")?)?;
    let nix_verify_value =
        crate::preserves_rail::parse_text(&read_output_text(output_path, "nix-dogfood-verify.preserves")?)?;
    let nix_evidence = parse_nix_dogfood_evidence(&nix_evidence_value)?;
    let nix_verify = parse_nix_dogfood_verify_receipt(&nix_verify_value)?;
    ensure_nix_release_artifacts_match(&observed_nix, &nix_evidence, &nix_verify)?;
    let mut member_refs = observed_nix.file_refs.clone();
    member_refs.push_limited_value(
        ("nix-dogfood-evidence.preserves".to_string(), nix_evidence.evidence_ref.clone()),
        MAX_OPERATOR_REFS,
        "release evidence bundle members",
    )?;
    member_refs.push_limited_value(
        ("nix-dogfood-verify.preserves".to_string(), nix_verify.receipt_ref.clone()),
        MAX_OPERATOR_REFS,
        "release evidence bundle members",
    )?;
    Ok(ObservedReleaseBundleOutput {
        output_path: observed_nix.output_path,
        output_path_ref: observed_nix.output_path_ref,
        report_ref: observed_nix.report_ref,
        release_gate_ref: observed_nix.release_gate_ref,
        replay_verify_ref: observed_nix.replay_verify_ref,
        replay_index_ref: observed_nix.replay_index_ref,
        nix_evidence_ref: nix_evidence.evidence_ref,
        nix_verify_ref: nix_verify.receipt_ref,
        summary_ref: observed_nix.summary_ref,
        nextest_marker_ref: observed_nix.nextest_marker_ref,
        nextest_check_path: observed_nix.nextest_check_path,
        member_refs,
    })
}

fn ensure_nix_release_artifacts_match(
    observed: &ObservedNixDogfoodOutput,
    evidence: &NixDogfoodEvidence,
    verify: &NixDogfoodVerifyReceipt,
) -> Result<()> {
    if let Some(mismatch) = [
        mismatch_diagnostic("Nix evidence output-path-ref", &evidence.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("Nix evidence report-ref", &evidence.report_ref, &observed.report_ref),
        mismatch_diagnostic("Nix evidence release-gate-ref", &evidence.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("Nix evidence replay-verify-ref", &evidence.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("Nix evidence replay-index-ref", &evidence.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("Nix evidence summary-ref", &evidence.summary_ref, &observed.summary_ref),
        mismatch_diagnostic(
            "Nix evidence nextest-marker-ref",
            &evidence.nextest_marker_ref,
            &observed.nextest_marker_ref,
        ),
        mismatch_diagnostic(
            "Nix evidence nextest-check-path",
            &evidence.nextest_check_path,
            &observed.nextest_check_path,
        ),
        mismatch_diagnostic("Nix verify evidence-ref", &verify.evidence_ref, &evidence.evidence_ref),
        mismatch_diagnostic("Nix verify output-path-ref", &verify.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("Nix verify report-ref", &verify.report_ref, &observed.report_ref),
        mismatch_diagnostic("Nix verify release-gate-ref", &verify.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("Nix verify replay-verify-ref", &verify.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("Nix verify replay-index-ref", &verify.replay_index_ref, &observed.replay_index_ref),
    ]
    .into_iter()
    .flatten()
    .next()
    {
        return Err(MoltenError::invalid_harness(mismatch));
    }
    if verify.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood verify receipt {} decision is {}",
            verify.receipt_ref, verify.decision
        )));
    }
    Ok(())
}

fn release_bundle_mismatch_diagnostics(
    bundle: &ReleaseEvidenceBundle,
    observed: &ObservedReleaseBundleOutput,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in [
        mismatch_diagnostic("output-path-ref", &bundle.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("report-ref", &bundle.report_ref, &observed.report_ref),
        mismatch_diagnostic("release-gate-ref", &bundle.release_gate_ref, &observed.release_gate_ref),
        mismatch_diagnostic("replay-verify-ref", &bundle.replay_verify_ref, &observed.replay_verify_ref),
        mismatch_diagnostic("replay-index-ref", &bundle.replay_index_ref, &observed.replay_index_ref),
        mismatch_diagnostic("nix-evidence-ref", &bundle.nix_evidence_ref, &observed.nix_evidence_ref),
        mismatch_diagnostic("nix-verify-ref", &bundle.nix_verify_ref, &observed.nix_verify_ref),
        mismatch_diagnostic("summary-ref", &bundle.summary_ref, &observed.summary_ref),
        mismatch_diagnostic("nextest-marker-ref", &bundle.nextest_marker_ref, &observed.nextest_marker_ref),
        mismatch_diagnostic("nextest-check-path", &bundle.nextest_check_path, &observed.nextest_check_path),
    ]
    .into_iter()
    .flatten()
    {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    for diagnostic in file_ref_mismatch_diagnostics(&bundle.member_refs, &observed.member_refs)? {
        diagnostics.push_limited_value(
            diagnostic,
            MAX_OPERATOR_DIAGNOSTICS,
            "release evidence bundle verify diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn release_bundle_signature_diagnostics(
    bundle: &ReleaseEvidenceBundle,
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.signed_member_values.is_empty() && !input.is_signed_members_required {
        return Ok(diagnostics);
    }
    let mut signable_members = Vec::new();
    for (name, member_ref) in &bundle.member_refs {
        if name.ends_with(".preserves") {
            signable_members.push_limited_value(
                (name.clone(), member_ref.clone()),
                MAX_OPERATOR_REFS,
                "release bundle signable member refs",
            )?;
        }
    }
    let mut signed_subject_refs = Vec::new();
    for signed_value in input.signed_member_values {
        match verify_release_bundle_signed_member(signed_value, input) {
            Ok(subject_ref) => {
                if signable_members.iter().any(|(_, member_ref)| member_ref == &subject_ref) {
                    if signed_subject_refs.iter().any(|known_ref| known_ref == &subject_ref) {
                        diagnostics.push_limited_value(
                            format!("duplicate signed member receipt for subject {subject_ref}"),
                            MAX_OPERATOR_DIAGNOSTICS,
                            "release evidence bundle signed member diagnostics",
                        )?;
                    }
                    signed_subject_refs.push_limited_value(
                        subject_ref,
                        MAX_OPERATOR_REFS,
                        "release evidence bundle signed member refs",
                    )?;
                } else {
                    diagnostics.push_limited_value(
                        format!("signed member subject {subject_ref} is not a signable bundle member"),
                        MAX_OPERATOR_DIAGNOSTICS,
                        "release evidence bundle signed member diagnostics",
                    )?;
                }
            }
            Err(error) => diagnostics.push_limited_value(
                format!("signed member verification failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release evidence bundle signed member diagnostics",
            )?,
        }
    }
    if input.is_signed_members_required {
        for (name, member_ref) in &signable_members {
            if !signed_subject_refs.iter().any(|subject_ref| subject_ref == member_ref) {
                diagnostics.push_limited_value(
                    format!("missing signed member receipt for {name}: {member_ref}"),
                    MAX_OPERATOR_DIAGNOSTICS,
                    "release evidence bundle signed member diagnostics",
                )?;
            }
        }
    }
    Ok(diagnostics)
}

fn verify_release_bundle_signed_member(
    signed_value: &IoValue,
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<String> {
    if input.signed_keys.is_empty() && input.signed_key_revocations.is_empty() {
        let signed = verify_signed_receipt_with_policy(signed_value, &VerifySignedReceiptPolicy {
            required_purpose: input.signed_purpose,
            trust_root: input.signed_trust_root,
            key: input.signed_key,
            expected_signer: input.signed_signer,
            expected_subject_ref: None,
        })?;
        Ok(signed.subject_ref)
    } else {
        let signed = verify_signed_receipt_with_keyring_policy(signed_value, &VerifySignedReceiptKeyringPolicy {
            required_purpose: input.signed_purpose,
            trust_root: input.signed_trust_root,
            expected_signer: input.signed_signer,
            expected_subject_ref: None,
            required_key_ref: input.signed_key_ref,
            required_key_id: input.signed_key_id,
            keys: input.signed_keys,
            revocations: input.signed_key_revocations,
        })?;
        Ok(signed.receipt.subject_ref)
    }
}

pub fn release_export_file_ref(name: &str, bytes: &[u8]) -> String {
    raw_bytes_ref("molten.operator.release-export.file.v1", name, bytes)
}

pub fn release_export_member_names() -> &'static [&'static str] {
    &[
        "dogfood-report.preserves",
        "dogfood-report.signed.preserves",
        "release-gate.preserves",
        "release-gate.signed.preserves",
        "replay-verify.preserves",
        "replay-verify.signed.preserves",
        "replay-evidence-index.preserves",
        "replay-evidence-index.signed.preserves",
        "dogfood-summary.txt",
        "after-nextest.txt",
        "nix-dogfood-evidence.preserves",
        "nix-dogfood-evidence.signed.preserves",
        "nix-dogfood-verify.preserves",
        "nix-dogfood-verify.signed.preserves",
        "nix-dogfood-verify.txt",
        "release-evidence-bundle.preserves",
        "release-evidence-bundle-verify.preserves",
        "release-evidence-bundle-verify.txt",
        "release-promotion-gate.preserves",
        "release-promotion-gate.txt",
        "release-promotion-gate.signed.preserves",
        "release-promotion-gate-signed-verify.txt",
        "release-promotion-summary.preserves",
        "release-promotion-summary.txt",
        "signed-keyring-import.txt",
    ]
}

fn observe_release_export_members(output_path: &Path) -> Result<Vec<(String, String)>> {
    let mut members = Vec::new();
    for name in release_export_member_names() {
        let bytes = std::fs::read(output_path.join(name)).map_err(MoltenError::from)?;
        members.push_limited_value(
            (name.to_string(), release_export_file_ref(name, &bytes)),
            MAX_OPERATOR_REFS,
            "release export members",
        )?;
    }
    for name in release_export_keyring_member_names(output_path)? {
        let bytes = std::fs::read(output_path.join(&name)).map_err(MoltenError::from)?;
        members.push_limited_value(
            (name.clone(), release_export_file_ref(&name, &bytes)),
            MAX_OPERATOR_REFS,
            "release export members",
        )?;
    }
    Ok(members)
}

fn release_export_keyring_member_names(output_path: &Path) -> Result<Vec<String>> {
    let keyring_path = output_path.join("signed-keyring");
    let mut names = Vec::with_capacity(MAX_OPERATOR_STEPS);
    let mut stack = Vec::with_capacity(MAX_OPERATOR_STEPS);
    stack.push_limited_value(
        (keyring_path, Path::new("signed-keyring").to_path_buf()),
        MAX_OPERATOR_REFS,
        "release export keyring traversal",
    )?;
    while let Some((path, relative)) = stack.pop() {
        for entry in std::fs::read_dir(path).map_err(MoltenError::from)? {
            let entry = entry.map_err(MoltenError::from)?;
            let child_path = entry.path();
            let child_relative = relative.join(entry.file_name());
            let file_type = entry.file_type().map_err(MoltenError::from)?;
            if file_type.is_dir() {
                stack.push_limited_value(
                    (child_path, child_relative),
                    MAX_OPERATOR_REFS,
                    "release export keyring traversal",
                )?;
            } else if file_type.is_file() {
                let name = child_relative.to_string_lossy().replace('\\', "/");
                names.push_limited_value(name, MAX_OPERATOR_REFS, "release export keyring members")?;
            }
        }
    }
    names.sort();
    Ok(names)
}

fn read_output_text(output_path: &Path, name: &str) -> Result<String> {
    std::fs::read_to_string(output_path.join(name)).map_err(MoltenError::from)
}

fn raw_text_ref(domain: &str, text: &str) -> String {
    let mut bytes = Vec::with_capacity(domain.len().saturating_add(text.len()).saturating_add(1));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(text.as_bytes());
    crate::preserves_rail::content_ref_from_bytes(&bytes)
}

fn raw_bytes_ref(domain: &str, name: &str, payload: &[u8]) -> String {
    let mut bytes =
        Vec::with_capacity(domain.len().saturating_add(name.len()).saturating_add(payload.len()).saturating_add(2));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload);
    crate::preserves_rail::content_ref_from_bytes(&bytes)
}

fn mismatch_diagnostic(label: &str, expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        None
    } else {
        Some(format!("{label} mismatch: evidence={expected} observed={actual}"))
    }
}

fn file_ref_mismatch_diagnostics(expected: &[(String, String)], observed: &[(String, String)]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if expected.len() != observed.len() {
        diagnostics.push_limited_value(
            format!("file ref count mismatch: evidence={} observed={}", expected.len(), observed.len()),
            MAX_OPERATOR_DIAGNOSTICS,
            "Nix dogfood verify diagnostics",
        )?;
    }
    for (expected_name, expected_ref) in expected {
        match observed.iter().find(|(observed_name, _)| observed_name == expected_name) {
            Some((_, observed_ref)) => {
                if let Some(diagnostic) = mismatch_diagnostic(expected_name, expected_ref, observed_ref) {
                    diagnostics.push_limited_value(
                        diagnostic,
                        MAX_OPERATOR_DIAGNOSTICS,
                        "Nix dogfood verify diagnostics",
                    )?;
                }
            }
            None => diagnostics.push_limited_value(
                format!("file ref missing from observed output: {expected_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?,
        }
    }
    for (observed_name, _) in observed {
        if !expected.iter().any(|(expected_name, _)| expected_name == observed_name) {
            diagnostics.push_limited_value(
                format!("unexpected observed file ref: {observed_name}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}

pub fn run_local_node_dogfood(input: &LocalNodeDogfoodInput<'_>) -> Result<LocalNodeDogfoodRun> {
    let state_root_ref = dogfood_ref("state-root")?;
    if let Some(dirty_reason) = dirty_state_reason(input.state_root)? {
        return dirty_state_report(&state_root_ref, dirty_reason);
    }
    let mut run = LocalRunState::new(input.state_root, state_root_ref)?;
    let start = run.record_start()?;
    let installed = run.record_install(&start.startup_ref)?;
    run.record_service()?;
    let remote = run.record_remote()?;
    let job = run.record_job()?;
    let retention_gc = run.record_gc()?;
    run.record_catalog(&installed, &remote.run.gate_receipt_value)?;
    run.record_repro(&remote.gate_ref)?;
    run.finish(&start, &installed, &job, &retention_gc)
}

struct LocalRunState<'a> {
    state_root: &'a Path,
    state_root_ref: String,
    registry_root: std::path::PathBuf,
    ledger_root: std::path::PathBuf,
    job_source_root: std::path::PathBuf,
    job_target_root: std::path::PathBuf,
    retention_root: std::path::PathBuf,
    bundle_root: std::path::PathBuf,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
    step_checkpoints: StepCheckpointBuffers,
    gate_receipt_refs: Vec<String>,
    repro_bundle_refs: Vec<String>,
    harness_gate_refs: Vec<String>,
    catalog_query_refs: Vec<String>,
    repro_verify_refs: Vec<String>,
    replay_index_refs: Vec<String>,
}

impl<'a> LocalRunState<'a> {
    fn new(state_root: &'a Path, state_root_ref: String) -> Result<Self> {
        std::fs::create_dir_all(state_root).map_err(MoltenError::from)?;
        Ok(Self {
            state_root,
            state_root_ref,
            registry_root: state_root.join("registry"),
            ledger_root: state_root.join("ledger"),
            job_source_root: state_root.join("job-source-registry"),
            job_target_root: state_root.join("job-target-registry"),
            retention_root: state_root.join("retention-store"),
            bundle_root: state_root.join("retention-bundle"),
            policy_refs: vec![dogfood_ref("operator-policy")?],
            capability_refs: vec![dogfood_ref("operator-capability")?],
            resource_refs: vec![dogfood_ref("operator-resource")?],
            step_checkpoints: StepCheckpointBuffers::default(),
            gate_receipt_refs: Vec::new(),
            repro_bundle_refs: Vec::new(),
            harness_gate_refs: Vec::new(),
            catalog_query_refs: Vec::new(),
            repro_verify_refs: Vec::new(),
            replay_index_refs: Vec::new(),
        })
    }

    fn record_start(&mut self) -> Result<StartSteps> {
        record_start_steps(StartStepInput {
            state_root: self.state_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_install(&mut self, startup_ref: &str) -> Result<crate::artifacts::ArtifactInstall> {
        record_install_step(InstallStepInput {
            registry_root: &self.registry_root,
            startup_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_service(&mut self) -> Result<()> {
        record_service_step(ServiceStepInput {
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_remote(&mut self) -> Result<RemoteStep> {
        let remote = record_remote_step(RemoteStepInput {
            state_root: self.state_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.gate_receipt_refs
            .push_limited_value(remote.gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
        Ok(remote)
    }

    fn record_job(&mut self) -> Result<JobRun> {
        record_job_step(JobStepInput {
            state_root: self.state_root,
            source: &self.job_source_root,
            target: &self.job_target_root,
            state_root_ref: &self.state_root_ref,
            policy_refs: &self.policy_refs,
            capability_refs: &self.capability_refs,
            resource_refs: &self.resource_refs,
            checkpoints: &mut self.step_checkpoints,
        })
    }

    fn record_gc(&mut self) -> Result<GcRun> {
        let retention_gc = record_gc_steps(GcStepInput {
            root: &self.retention_root,
            bundle_dir: &self.bundle_root,
            ledger_root: &self.ledger_root,
            registry_root: &self.registry_root,
            state_root_ref: &self.state_root_ref,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.catalog_query_refs.push_limited_value(
            retention_gc.catalog_receipt_ref.clone(),
            MAX_OPERATOR_REFS,
            "catalog query refs",
        )?;
        Ok(retention_gc)
    }

    fn record_catalog(
        &mut self,
        installed: &crate::artifacts::ArtifactInstall,
        remote_gate_value: &IoValue,
    ) -> Result<()> {
        let mcp_receipt_ref = record_catalog_step(CatalogStepInput {
            ledger_root: &self.ledger_root,
            registry_root: &self.registry_root,
            state_root_ref: &self.state_root_ref,
            installed,
            remote_gate_value,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.catalog_query_refs.push_limited_value(mcp_receipt_ref, MAX_OPERATOR_REFS, "catalog query refs")
    }

    fn record_repro(&mut self, remote_gate_ref: &str) -> Result<()> {
        let repro = record_repro_steps(ReproStepInput {
            state_root_ref: &self.state_root_ref,
            remote_gate_ref,
            checkpoints: &mut self.step_checkpoints,
        })?;
        self.harness_gate_refs
            .push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "harness gate refs")?;
        self.gate_receipt_refs
            .push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
        self.repro_bundle_refs
            .push_limited_value(repro.bundle_ref.clone(), MAX_OPERATOR_REFS, "dogfood repro refs")?;
        self.repro_verify_refs
            .push_limited_value(repro.verify_ref, MAX_OPERATOR_REFS, "dogfood repro verify refs")
    }

    fn finish(
        self,
        start: &StartSteps,
        installed: &crate::artifacts::ArtifactInstall,
        job: &JobRun,
        retention_gc: &GcRun,
    ) -> Result<LocalNodeDogfoodRun> {
        let Self {
            state_root_ref,
            ledger_root,
            policy_refs,
            capability_refs,
            resource_refs,
            step_checkpoints,
            gate_receipt_refs,
            repro_bundle_refs,
            harness_gate_refs,
            catalog_query_refs,
            repro_verify_refs,
            replay_index_refs,
            ..
        } = self;
        finish_run(FinishInput {
            ledger_root: &ledger_root,
            state_root_ref: &state_root_ref,
            startup_ref: &start.startup_ref,
            node_started: &start.node_started,
            installed,
            job,
            retention_gc,
            step_checkpoints,
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            gate_receipt_refs,
            repro_bundle_refs,
            harness_gate_refs,
            catalog_query_refs,
            repro_verify_refs,
            replay_index_refs,
        })
    }
}

pub fn operator_dogfood_summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = base_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = evidence_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = promotion_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = export_summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported operator dogfood artifact for summary"))
}

fn base_summary(value: &IoValue) -> Option<String> {
    if let Ok(report) = parse_dogfood_report(value) {
        return Some(format!(
            "operator dogfood report ref={} decision={} workflow={} final_state={} steps={} gates={} repro={} diagnostics={} (summary is non-normative)",
            report.report_ref,
            report.decision,
            report.workflow_ref,
            report.final_state_ref,
            report.step_receipts.len(),
            report.gate_receipts.len(),
            report.repro_bundles.len(),
            report.diagnostics.len()
        ));
    }
    if let Ok(workflow) = parse_operator_workflow(value) {
        return Some(format!(
            "operator workflow ref={} id={} steps={} replay={} (summary is non-normative)",
            workflow.workflow_ref,
            workflow.workflow_id,
            workflow.steps.len(),
            workflow.replay_profile
        ));
    }
    if let Ok(checkpoint) = parse_operator_checkpoint(value) {
        return Some(format!(
            "operator checkpoint ref={} workflow={} sequence={} step={} receipt={} (summary is non-normative)",
            checkpoint.checkpoint_ref,
            checkpoint.workflow_id,
            checkpoint.sequence,
            checkpoint.step_ref,
            checkpoint.receipt_ref.as_deref().unwrap_or("none")
        ));
    }
    if let Ok(receipt) = parse_release_gate_receipt(value) {
        return Some(format!(
            "operator release gate receipt ref={} decision={} report={} checks={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.report_ref,
            receipt.checks.len()
        ));
    }
    None
}

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

fn resolve_identity(state_root: &Path, policy_refs: &[String]) -> Result<crate::node_identity::NodeIdentityResolution> {
    let mut config = crate::node_identity::NodeIdentityConfig::new("node:dogfood-local", state_root.join("identity"));
    config.policy_refs = policy_refs.to_vec();
    crate::node_identity::resolve_node_identity(&config)
}

fn start_node(
    identity: &crate::node_identity::NodeIdentity,
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
        node_identity_ref: &identity.identity_ref,
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

struct GcStepInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    ledger_root: &'a Path,
    registry_root: &'a Path,
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_gc_steps(input: GcStepInput<'_>) -> Result<GcRun> {
    let GcStepInput {
        root,
        bundle_dir,
        ledger_root,
        registry_root,
        state_root_ref,
        checkpoints,
    } = input;
    let retention_gc = run_gc_workflow(GcWorkflowInput {
        root,
        bundle_dir,
        ledger_root,
        registry_root,
    })?;
    record_gc_plan_steps(checkpoints, state_root_ref, &retention_gc)?;
    record_gc_review_steps(checkpoints, state_root_ref, &retention_gc)?;
    Ok(retention_gc)
}

fn record_gc_plan_steps(
    checkpoints: &mut StepCheckpointBuffers,
    state_root_ref: &str,
    retention_gc: &GcRun,
) -> Result<()> {
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "plan-retention-gc",
        request_ref: Some(&retention_gc.object_ref),
        receipt_ref: Some(&retention_gc.plan_ref),
        result_ref: Some(&retention_gc.plan_ref),
        decision: &retention_gc.plan_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.plan_ref),
        diagnostics: &retention_gc.plan_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "apply-retention-gc-plan",
        request_ref: Some(&retention_gc.plan_ref),
        receipt_ref: Some(&retention_gc.apply_ref),
        result_ref: Some(&retention_gc.apply_ref),
        decision: &retention_gc.apply_decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.apply_ref),
        diagnostics: &retention_gc.apply_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "execute-retention-gc",
        request_ref: Some(&retention_gc.apply_ref),
        receipt_ref: Some(&retention_gc.execution_ref),
        result_ref: Some(&retention_gc.execution_ref),
        decision: &retention_gc.execution_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.execution_ref),
        diagnostics: &retention_gc.execution_diagnostics,
        state_root_ref,
    })
}

fn record_gc_review_steps(
    checkpoints: &mut StepCheckpointBuffers,
    state_root_ref: &str,
    retention_gc: &GcRun,
) -> Result<()> {
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "audit-retention-gc",
        request_ref: Some(&retention_gc.execution_ref),
        receipt_ref: Some(&retention_gc.audit_ref),
        result_ref: Some(&retention_gc.audit_ref),
        decision: &retention_gc.audit_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.audit_ref),
        diagnostics: &retention_gc.audit_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "export-retention-gc-bundle",
        request_ref: Some(&retention_gc.explain_ref),
        receipt_ref: Some(&retention_gc.bundle_verify_ref),
        result_ref: Some(&retention_gc.bundle_ref),
        decision: &retention_gc.bundle_verify_decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[
            retention_gc.bundle_ref.clone(),
            retention_gc.bundle_profile_ref.clone(),
            retention_gc.bundle_verify_ref.clone(),
        ],
        diagnostics: &retention_gc.bundle_diagnostics,
        state_root_ref,
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "search-retention-gc-catalog",
        request_ref: Some(&retention_gc.catalog_request_ref),
        receipt_ref: Some(&retention_gc.catalog_receipt_ref),
        result_ref: Some(&retention_gc.catalog_response_ref),
        decision: &retention_gc.catalog_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &retention_gc.artifact_refs,
        diagnostics: &[],
        state_root_ref,
    })
}

struct FinishInput<'a> {
    ledger_root: &'a Path,
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    retention_gc: &'a GcRun,
    step_checkpoints: StepCheckpointBuffers,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    gate_receipt_refs: Vec<String>,
    repro_bundle_refs: Vec<String>,
    harness_gate_refs: Vec<String>,
    catalog_query_refs: Vec<String>,
    repro_verify_refs: Vec<String>,
    replay_index_refs: Vec<String>,
}

struct ReplayShutdownInput<'a> {
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    step_checkpoints: StepCheckpointBuffers,
    replay_index_refs: Vec<String>,
}

struct ReplayShutdown {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    shutdown_ref: String,
    health_ref: String,
    step_checkpoints: StepCheckpointBuffers,
    replay_index_refs: Vec<String>,
}

struct ReplayStep {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    replay_index_refs: Vec<String>,
}

struct ShutdownStepInput<'a> {
    state_root_ref: &'a str,
    startup_ref: &'a str,
    node_started: &'a crate::node_runtime::NodeRuntimeStart,
    installed: &'a crate::artifacts::ArtifactInstall,
    job: &'a JobRun,
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct ShutdownStep {
    shutdown_ref: String,
    health_ref: String,
}

fn record_replay_shutdown(input: ReplayShutdownInput<'_>) -> Result<ReplayShutdown> {
    let ReplayShutdownInput {
        state_root_ref,
        startup_ref,
        node_started,
        installed,
        job,
        mut step_checkpoints,
        replay_index_refs,
    } = input;
    let replay = record_replay_step(state_root_ref, &mut step_checkpoints, replay_index_refs)?;
    let shutdown = record_shutdown_step(ShutdownStepInput {
        state_root_ref,
        startup_ref,
        node_started,
        installed,
        job,
        checkpoints: &mut step_checkpoints,
    })?;
    Ok(ReplayShutdown {
        replay_verify: replay.replay_verify,
        replay_index: replay.replay_index,
        shutdown_ref: shutdown.shutdown_ref,
        health_ref: shutdown.health_ref,
        step_checkpoints,
        replay_index_refs: replay.replay_index_refs,
    })
}

fn record_replay_step(
    state_root_ref: &str,
    checkpoints: &mut StepCheckpointBuffers,
    mut replay_index_refs: Vec<String>,
) -> Result<ReplayStep> {
    let replay_verify =
        crate::deterministic_replay::verify_fixture_value(crate::deterministic_replay::ReplayFixtureVariant::Baseline)?;
    let replay_index =
        crate::deterministic_replay::index_replay_evidence(&[crate::deterministic_replay::ReplayIndexInput {
            expected_ref: Some(replay_verify.receipt_ref.clone()),
            value: replay_verify.value.clone(),
        }])?;
    replay_index_refs.push_limited_value(
        replay_index.index_ref.clone(),
        MAX_OPERATOR_REFS,
        "dogfood replay index refs",
    )?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "index-replay-evidence",
        request_ref: Some(&replay_verify.receipt_ref),
        receipt_ref: Some(&replay_index.index_ref),
        result_ref: Some(&replay_index.index_ref),
        decision: &replay_index.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&replay_verify.receipt_ref),
        diagnostics: &[],
        state_root_ref,
    })?;
    Ok(ReplayStep {
        replay_verify,
        replay_index,
        replay_index_refs,
    })
}

fn record_shutdown_step(input: ShutdownStepInput<'_>) -> Result<ShutdownStep> {
    let shutdown = crate::node_runtime::node_shutdown_receipt_value(&crate::node_runtime::ShutdownReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: input.startup_ref,
        adapter_receipts: &input.node_started.adapter_receipts,
        drained_job_refs: std::slice::from_ref(&input.job.execution_receipt_ref),
        index_receipt_refs: &[dogfood_ref("shutdown-index")?],
        diagnostics: &[],
    })?;
    let shutdown_ref = crate::preserves_rail::canonical_hash(&shutdown)?;
    let health =
        crate::node_runtime::node_restart_health_receipt_value(&crate::node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &input.node_started.startup_receipt,
            shutdown_receipt_ref: Some(&shutdown_ref),
            index_receipt_refs: &[dogfood_ref("restart-health-index")?],
            head_refs: &[
                input.installed.artifact_ref.clone(),
                input.job.execution_receipt_ref.clone(),
            ],
            open_job_refs: &[],
            diagnostics: &[],
        })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "shutdown-node",
        request_ref: Some(input.startup_ref),
        receipt_ref: Some(&shutdown_ref),
        result_ref: Some(&health_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&health_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(ShutdownStep {
        shutdown_ref,
        health_ref,
    })
}

struct FinishReportInput<'a> {
    state_root_ref: &'a str,
    shutdown_ref: &'a str,
    health_ref: &'a str,
    checkpoints: &'a StepCheckpointBuffers,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    gate_receipt_refs: &'a [String],
    repro_bundle_refs: &'a [String],
}

struct FinishReport {
    workflow_value: IoValue,
    report_value: IoValue,
    report: DogfoodReport,
}

fn build_finish_report(input: FinishReportInput<'_>) -> Result<FinishReport> {
    let workflow_value = operator_workflow_value(&OperatorWorkflowInput {
        workflow_id: LOCAL_NODE_WORKFLOW_ID,
        steps: &input.checkpoints.steps,
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        resource_refs: input.resource_refs,
        replay_profile: "recorded",
    })?;
    let final_state_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("operator-dogfood-final-state", vec![
            crate::preserves_rail::string(input.state_root_ref),
            crate::preserves_rail::string(input.shutdown_ref),
            crate::preserves_rail::string(input.health_ref),
        ]))?;
    let report_value = dogfood_report_value(&DogfoodReportInput {
        workflow_value: &workflow_value,
        checkpoint_values: &input.checkpoints.checkpoints,
        gate_receipt_refs: input.gate_receipt_refs,
        repro_bundle_refs: input.repro_bundle_refs,
        final_state_ref: &final_state_ref,
        diagnostics: &[],
    })?;
    let report = parse_dogfood_report(&report_value)?;
    Ok(FinishReport {
        workflow_value,
        report_value,
        report,
    })
}

struct ReleaseValueInput<'a> {
    report: &'a DogfoodReport,
    report_value: &'a IoValue,
    startup_ref: &'a str,
    shutdown_ref: &'a str,
    harness_gate_refs: &'a [String],
    catalog_query_refs: &'a [String],
    repro_verify_refs: &'a [String],
    replay_index_refs: &'a [String],
    retention_gc: &'a GcRun,
}

fn build_release_value(input: ReleaseValueInput<'_>) -> Result<Option<IoValue>> {
    let validation_command_refs = vec![dogfood_ref("cargo-nextest-ci")?];
    let gc_release_refs = vec![
        input.retention_gc.audit_ref.clone(),
        input.retention_gc.bundle_verify_ref.clone(),
        input.retention_gc.catalog_receipt_ref.clone(),
    ];
    if input.report.decision == "pass" {
        Ok(Some(release_gate_receipt_value(&ReleaseGateInput {
            report_value: input.report_value,
            node_startup_ref: input.startup_ref,
            node_shutdown_ref: input.shutdown_ref,
            harness_gate_refs: input.harness_gate_refs,
            catalog_query_refs: input.catalog_query_refs,
            repro_verify_refs: input.repro_verify_refs,
            replay_index_refs: input.replay_index_refs,
            gc_refs: &gc_release_refs,
            validation_command_refs: &validation_command_refs,
        })?))
    } else {
        Ok(None)
    }
}

struct FinishReplay {
    replay_verify: crate::deterministic_replay::ReplayVerifyReceipt,
    replay_index: crate::deterministic_replay::ReplayIndexReceipt,
    shutdown_ref: String,
    health_ref: String,
}

struct FinishState<'a> {
    input: FinishInput<'a>,
}

impl<'a> FinishState<'a> {
    fn record_replay(&mut self) -> Result<FinishReplay> {
        let replay = record_replay_shutdown(ReplayShutdownInput {
            state_root_ref: self.input.state_root_ref,
            startup_ref: self.input.startup_ref,
            node_started: self.input.node_started,
            installed: self.input.installed,
            job: self.input.job,
            step_checkpoints: std::mem::take(&mut self.input.step_checkpoints),
            replay_index_refs: std::mem::take(&mut self.input.replay_index_refs),
        })?;
        self.input.step_checkpoints = replay.step_checkpoints;
        self.input.replay_index_refs = replay.replay_index_refs;
        Ok(FinishReplay {
            replay_verify: replay.replay_verify,
            replay_index: replay.replay_index,
            shutdown_ref: replay.shutdown_ref,
            health_ref: replay.health_ref,
        })
    }

    fn build_report(&self, replay: &FinishReplay) -> Result<FinishReport> {
        build_finish_report(FinishReportInput {
            state_root_ref: self.input.state_root_ref,
            shutdown_ref: &replay.shutdown_ref,
            health_ref: &replay.health_ref,
            checkpoints: &self.input.step_checkpoints,
            policy_refs: self.input.policy_refs,
            capability_refs: self.input.capability_refs,
            resource_refs: self.input.resource_refs,
            gate_receipt_refs: &self.input.gate_receipt_refs,
            repro_bundle_refs: &self.input.repro_bundle_refs,
        })
    }

    fn build_release(&self, replay: &FinishReplay, finish_report: &FinishReport) -> Result<Option<IoValue>> {
        build_release_value(ReleaseValueInput {
            report: &finish_report.report,
            report_value: &finish_report.report_value,
            startup_ref: self.input.startup_ref,
            shutdown_ref: &replay.shutdown_ref,
            harness_gate_refs: &self.input.harness_gate_refs,
            catalog_query_refs: &self.input.catalog_query_refs,
            repro_verify_refs: &self.input.repro_verify_refs,
            replay_index_refs: &self.input.replay_index_refs,
            retention_gc: self.input.retention_gc,
        })
    }

    fn import_evidence(
        &self,
        replay: &FinishReplay,
        finish_report: &FinishReport,
        release_gate_value: Option<&IoValue>,
    ) -> Result<Vec<String>> {
        import_dogfood_evidence(DogfoodEvidenceImportInput {
            ledger_root: self.input.ledger_root,
            workflow_value: &finish_report.workflow_value,
            step_values: &self.input.step_checkpoints.steps,
            checkpoint_values: &self.input.step_checkpoints.checkpoints,
            report_value: &finish_report.report_value,
            release_gate_value,
            replay_verify_value: &replay.replay_verify.value,
            replay_index_value: &replay.replay_index.value,
        })
    }

    fn complete(
        self,
        replay: FinishReplay,
        finish_report: FinishReport,
        release_gate_value: Option<IoValue>,
        import_refs: Vec<String>,
    ) -> Result<LocalNodeDogfoodRun> {
        let release_gate_ref = release_gate_value.as_ref().map(crate::preserves_rail::canonical_hash).transpose()?;
        let StepCheckpointBuffers { steps, checkpoints } = self.input.step_checkpoints;
        Ok(LocalNodeDogfoodRun {
            decision: finish_report.report.decision,
            workflow_ref: crate::preserves_rail::canonical_hash(&finish_report.workflow_value)?,
            workflow_value: finish_report.workflow_value,
            step_values: steps,
            checkpoint_values: checkpoints,
            report_ref: finish_report.report.report_ref,
            report_value: finish_report.report_value,
            release_gate_ref,
            release_gate_value,
            replay_verify_ref: Some(replay.replay_verify.receipt_ref),
            replay_verify_value: Some(replay.replay_verify.value),
            replay_index_ref: Some(replay.replay_index.index_ref),
            replay_index_value: Some(replay.replay_index.value),
            ledger_import_receipt_refs: import_refs,
        })
    }
}

fn finish_run(input: FinishInput<'_>) -> Result<LocalNodeDogfoodRun> {
    let mut finish = FinishState { input };
    let replay = finish.record_replay()?;
    let finish_report = finish.build_report(&replay)?;
    let release_gate_value = finish.build_release(&replay, &finish_report)?;
    let import_refs = finish.import_evidence(&replay, &finish_report, release_gate_value.as_ref())?;
    finish.complete(replay, finish_report, release_gate_value, import_refs)
}

struct StartStepInput<'a> {
    state_root: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct StartSteps {
    node_started: crate::node_runtime::NodeRuntimeStart,
    startup_ref: String,
}

fn record_start_steps(input: StartStepInput<'_>) -> Result<StartSteps> {
    let StartStepInput {
        state_root,
        state_root_ref,
        policy_refs,
        capability_refs,
        resource_refs,
        checkpoints,
    } = input;
    let identity_resolution = resolve_identity(state_root, policy_refs)?;
    let identity = identity_resolution
        .identity
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("local dogfood identity resolution denied"))?;
    let identity_startup = crate::node_identity::node_identity_startup_evidence_value(
        &identity.identity_ref,
        &identity_resolution.receipt_ref,
    )?;
    let identity_startup_ref = crate::preserves_rail::canonical_hash(&identity_startup)?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "clean-state",
        request_ref: Some(state_root_ref),
        receipt_ref: Some(&identity_resolution.receipt_ref),
        result_ref: Some(&identity_startup_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[identity.identity_ref.clone(), identity_startup_ref.clone()],
        diagnostics: &[],
        state_root_ref,
    })?;

    let node_started =
        start_node(&identity, &identity_resolution.receipt_ref, policy_refs, capability_refs, resource_refs)?;
    let startup_ref = node_started.startup_receipt.receipt_ref.clone();
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "start-node",
        request_ref: Some(&node_started.config.config_ref),
        receipt_ref: Some(&startup_ref),
        result_ref: Some(&startup_ref),
        decision: &node_started.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&node_started.config.config_ref),
        diagnostics: &node_started.startup_receipt.diagnostics,
        state_root_ref,
    })?;
    Ok(StartSteps {
        node_started,
        startup_ref,
    })
}

struct InstallStepInput<'a> {
    registry_root: &'a Path,
    startup_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_install_step(input: InstallStepInput<'_>) -> Result<crate::artifacts::ArtifactInstall> {
    let InstallStepInput {
        registry_root,
        startup_ref,
        policy_refs,
        capability_refs,
        state_root_ref,
        checkpoints,
    } = input;
    let installed = crate::artifacts::install_artifact(registry_root, &crate::artifacts::ArtifactInstallInput {
        kind: "operator-artifact".to_string(),
        payload: crate::preserves_rail::record("dogfood-artifact", vec![crate::preserves_rail::string("local-node")]),
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![startup_ref.to_string()],
        installer_ref: capability_refs[0].clone(),
        capability_refs: capability_refs.to_vec(),
    })?;
    push_step_checkpoint(checkpoints, StepCheckpointInput {
        name: "install-artifact",
        request_ref: Some(startup_ref),
        receipt_ref: Some(&crate::preserves_rail::canonical_hash(&installed.receipt_value)?),
        result_ref: Some(&installed.artifact_ref),
        decision: &installed.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&installed.artifact_ref),
        diagnostics: &[],
        state_root_ref,
    })?;
    Ok(installed)
}

struct ServiceStepInput<'a> {
    state_root_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_service_step(input: ServiceStepInput<'_>) -> Result<()> {
    let service_suite = crate::service_runtime::two_service_suite_value()?;
    let service_run = crate::service_runtime::run_service_runtime_suite_value(&service_suite)?;
    let service_decision = if service_run.lifecycle_receipts.iter().all(service_lifecycle_pass) {
        "pass"
    } else {
        "deny"
    };
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "start-service",
        request_ref: Some(&crate::preserves_rail::canonical_hash(&service_suite)?),
        receipt_ref: Some(&service_run.report_ref),
        result_ref: Some(&service_run.report_ref),
        decision: service_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &service_run
            .readiness_assertions
            .iter()
            .map(crate::preserves_rail::canonical_hash)
            .collect::<Result<Vec<_>>>()?,
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })
}

struct RemoteStepInput<'a> {
    state_root: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

struct RemoteStep {
    run: crate::remote_dataspace::RemoteTwoPeerHarness,
    gate_ref: String,
}

fn record_remote_step(input: RemoteStepInput<'_>) -> Result<RemoteStep> {
    let remote = crate::remote_dataspace::two_peer_service_ready_harness(
        &input.state_root.join("remote-dataspace"),
        crate::remote_dataspace::RemoteDeliveryEvidence {
            peer_bootstrap_refs: vec![dogfood_ref("remote-peer-bootstrap")?],
            capability_refs: vec![dogfood_ref("remote-capability")?],
            policy_refs: input.policy_refs.to_vec(),
            resource_refs: input.resource_refs.to_vec(),
            authority_refs: vec![dogfood_ref("remote-authority")?],
        },
    )?;
    let gate_ref = crate::preserves_rail::canonical_hash(&remote.gate_receipt_value)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "publish-remote-assertion",
        request_ref: Some(&remote.delivery_log.log_ref),
        receipt_ref: Some(&gate_ref),
        result_ref: Some(&gate_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&remote.delivery_log.log_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(RemoteStep { run: remote, gate_ref })
}

struct JobStepInput<'a> {
    state_root: &'a Path,
    source: &'a Path,
    target: &'a Path,
    state_root_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_job_step(input: JobStepInput<'_>) -> Result<JobRun> {
    let job = run_job_stack(JobStackInput {
        state_root: input.state_root,
        source: input.source,
        target: input.target,
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        resource_refs: input.resource_refs,
    })?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "run-job-dag",
        request_ref: Some(&job.execution_request_ref),
        receipt_ref: Some(&job.execution_receipt_ref),
        result_ref: Some(&job.execution_receipt_ref),
        decision: &job.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &job.artifact_refs,
        diagnostics: &job.diagnostics,
        state_root_ref: input.state_root_ref,
    })?;
    Ok(job)
}

struct CatalogStepInput<'a> {
    ledger_root: &'a Path,
    registry_root: &'a Path,
    state_root_ref: &'a str,
    installed: &'a crate::artifacts::ArtifactInstall,
    remote_gate_value: &'a IoValue,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_catalog_step(input: CatalogStepInput<'_>) -> Result<String> {
    crate::ledger::import_artifact(input.ledger_root, &input.installed.artifact.value)?;
    crate::ledger::import_artifact(input.ledger_root, input.remote_gate_value)?;
    let mcp_request =
        crate::catalog_mcp::mcp_request_value("catalog.list", vec![crate::preserves_rail::record("kind", vec![
            crate::preserves_rail::string("operator-artifact"),
        ])])?;
    let mcp_call = crate::catalog_mcp::call(input.registry_root, Some(input.ledger_root), &mcp_request)?;
    let mcp_receipt_ref = crate::preserves_rail::canonical_hash(&mcp_call.receipt_value)?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "query-catalog-mcp",
        request_ref: Some(&mcp_call.request.request_ref),
        receipt_ref: Some(&mcp_receipt_ref),
        result_ref: Some(&mcp_call.response_ref),
        decision: &mcp_call.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&mcp_call.response_ref),
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(mcp_receipt_ref)
}

struct ReproStepInput<'a> {
    state_root_ref: &'a str,
    remote_gate_ref: &'a str,
    checkpoints: &'a mut StepCheckpointBuffers,
}

fn record_repro_steps(input: ReproStepInput<'_>) -> Result<DogfoodRepro> {
    let repro = build_dogfood_repro()?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "export-redacted-repro",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.verify_ref),
        result_ref: Some(&repro.bundle_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[repro.gate_ref.clone(), repro.bundle_ref.clone()],
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    push_step_checkpoint(input.checkpoints, StepCheckpointInput {
        name: "gate-evidence",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.gate_ref),
        result_ref: Some(&repro.gate_ref),
        decision: "pass",
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &[input.remote_gate_ref.to_string()],
        diagnostics: &[],
        state_root_ref: input.state_root_ref,
    })?;
    Ok(repro)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetentionAdmissionFixtureInput<'a> {
    root: &'a Path,
    kind: &'a str,
    label: &'a str,
    requester_ref: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
    remote_refs: &'a [String],
}

struct JobParts {
    job_ref: String,
    provenance_values: Vec<IoValue>,
}

struct StageArtifacts {
    base_ref: String,
    source_ref: String,
    map_ref: String,
}

struct JobSyncInput<'a> {
    source: &'a Path,
    target: &'a Path,
    parts: &'a JobParts,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
}

struct JobAdmissionParts {
    authority_ref: String,
    receipt_ref: String,
    receipt_value: IoValue,
    stage_order: Vec<String>,
}

struct JobAdmissionInput<'a> {
    target: &'a Path,
    job_ref: &'a str,
    sync_ref: &'a str,
    policy_refs: &'a [String],
    capability_refs: &'a [String],
    resource_refs: &'a [String],
}

struct JobExecutionParts {
    request_ref: String,
    receipt_ref: String,
    decision: String,
    diagnostics: Vec<String>,
    output_refs: Vec<String>,
}

struct JobExecutionInput<'a> {
    state_root: &'a Path,
    target: &'a Path,
    job_ref: &'a str,
    admission: &'a JobAdmissionParts,
    policy_refs: &'a [String],
    resource_refs: &'a [String],
}

fn run_job_stack(input: JobStackInput<'_>) -> Result<JobRun> {
    let JobStackInput {
        state_root,
        source,
        target,
        policy_refs,
        capability_refs,
        resource_refs,
    } = input;
    let parts = install_job_parts(source, policy_refs, capability_refs)?;
    let sync_ref = sync_job_stack(JobSyncInput {
        source,
        target,
        parts: &parts,
        policy_refs,
        capability_refs,
    })?;
    let admission = admit_job_stack(JobAdmissionInput {
        target,
        job_ref: &parts.job_ref,
        sync_ref: &sync_ref,
        policy_refs,
        capability_refs,
        resource_refs,
    })?;
    let execution = execute_job_stack(JobExecutionInput {
        state_root,
        target,
        job_ref: &parts.job_ref,
        admission: &admission,
        policy_refs,
        resource_refs,
    })?;
    let mut artifact_refs = vec![
        parts.job_ref,
        sync_ref,
        admission.receipt_ref,
        admission.authority_ref,
        execution.request_ref.clone(),
    ];
    artifact_refs.extend(execution.output_refs);
    Ok(JobRun {
        execution_request_ref: execution.request_ref,
        execution_receipt_ref: execution.receipt_ref,
        decision: execution.decision,
        diagnostics: execution.diagnostics,
        artifact_refs,
    })
}

fn install_job_parts(source: &Path, policy_refs: &[String], capability_refs: &[String]) -> Result<JobParts> {
    let stages = install_stage_artifacts(source, policy_refs, capability_refs)?;
    let dag = job_graph_value(&stages, policy_refs)?;
    let installed = crate::job_dag::install_job_dag(source, &dag)?;
    let provenance_refs = vec![
        stages.base_ref,
        stages.source_ref,
        stages.map_ref,
        installed.artifact_ref,
    ];
    Ok(JobParts {
        job_ref: installed.job_ref,
        provenance_values: provenance_values(&provenance_refs)?,
    })
}

fn install_stage_artifacts(
    source: &Path,
    policy_refs: &[String],
    capability_refs: &[String],
) -> Result<StageArtifacts> {
    let base = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "schema".to_string(),
        payload: crate::preserves_rail::record("schema", vec![crate::preserves_rail::string("dogfood-job-base")]),
        schema_refs: vec![dogfood_ref("job-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-evidence")?],
        installer_ref: dogfood_ref("job-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let source_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("source")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let map_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("identity")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: vec![base.artifact_ref.clone()],
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(StageArtifacts {
        base_ref: base.artifact_ref,
        source_ref: source_stage.artifact_ref,
        map_ref: map_stage.artifact_ref,
    })
}

fn job_graph_value(stages: &StageArtifacts, policy_refs: &[String]) -> Result<IoValue> {
    let source_node = crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: Some(&stages.source_ref),
        input_ports: &[],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
            crate::preserves_rail::sequence(vec![crate::preserves_rail::string("dogfood-job")]),
        ])]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let map_node = crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "map",
        kind: "map",
        stage_artifact_ref: Some(&stages.map_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let edge = crate::job_dag::job_edge_value(crate::job_dag::EdgeValueInput {
        from_node: "source",
        from_port: "out",
        to_node: "map",
        to_port: "in",
        schema_ref: None,
        partitioning: "single",
        materialization: "stream",
    })?;
    crate::job_dag::job_dag_value(crate::job_dag::DagValueInput {
        nodes: vec![source_node, map_node],
        edges: vec![edge],
        output_roots: &["map".to_string()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs,
        evidence_refs: std::slice::from_ref(&stages.base_ref),
    })
}

fn provenance_values(artifact_refs: &[String]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(artifact_refs.len());
    for artifact_ref in artifact_refs {
        values.push_limited_value(
            crate::provenance::synthetic_reviewed_provenance_record(artifact_ref)?,
            MAX_OPERATOR_REFS,
            "dogfood sync provenance",
        )?;
    }
    Ok(values)
}

fn sync_job_stack(input: JobSyncInput<'_>) -> Result<String> {
    let sync_request = crate::job_dag::job_sync_request_value(crate::job_dag::SyncRequestValueInput {
        job_ref: &input.parts.job_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs: input.policy_refs,
        capability_refs: input.capability_refs,
        evidence_refs: &[dogfood_ref("job-sync-evidence")?],
    })?;
    let sync = crate::job_dag::sync_loopback(crate::job_dag::SyncLoopbackInput {
        source_registry: input.source,
        target_registry: input.target,
        request_value: &sync_request,
        provenance_values: &input.parts.provenance_values,
        build_verification_values: &[],
    })?;
    crate::preserves_rail::canonical_hash(&sync.receipt_value)
}

fn admit_job_stack(input: JobAdmissionInput<'_>) -> Result<JobAdmissionParts> {
    let authority_ref =
        install_job_execute_authority_context(input.target, input.job_ref, input.policy_refs, input.capability_refs)?;
    let source_gate_ref = install_clean_octet_gate(input.target, input.policy_refs, input.capability_refs)?;
    let admission_request = crate::job_dag::job_admission_request_value(crate::job_dag::AdmissionRequestValueInput {
        job_ref: input.job_ref,
        sync_ref: input.sync_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs: input.policy_refs,
        capability_refs: std::slice::from_ref(&authority_ref),
        evidence_refs: &[input.sync_ref.to_string(), source_gate_ref],
        resource_refs: input.resource_refs,
    })?;
    let admission = crate::job_dag::admission_loopback(input.target, &admission_request)?;
    Ok(JobAdmissionParts {
        authority_ref,
        receipt_ref: crate::preserves_rail::canonical_hash(&admission.receipt_value)?,
        receipt_value: admission.receipt_value,
        stage_order: admission.plan.stage_order,
    })
}

fn execute_job_stack(input: JobExecutionInput<'_>) -> Result<JobExecutionParts> {
    let execution_request = crate::job_dag::job_execution_request_value(crate::job_dag::ExecutionRequestValueInput {
        job_ref: input.job_ref,
        admission_ref: &input.admission.receipt_ref,
        stage_ids: &input.admission.stage_order,
        target_peer: "peer:dogfood",
        storage_profile_ref: &dogfood_ref("job-storage-profile")?,
        cache_profile_ref: &dogfood_ref("job-cache-profile")?,
        chunk_profile_ref: &dogfood_ref("job-chunk-profile")?,
        policy_refs: input.policy_refs,
        capability_refs: std::slice::from_ref(&input.admission.authority_ref),
        resource_refs: input.resource_refs,
    })?;
    let request_ref = crate::preserves_rail::canonical_hash(&execution_request)?;
    let execution = crate::job_dag::execution_loopback(crate::job_dag::ExecutionLoopbackInput {
        target_registry: input.target,
        storage_root: &input.state_root.join("job-storage"),
        cache_root: &input.state_root.join("job-cache"),
        chunk_root: &input.state_root.join("job-chunks"),
        admission_receipt_value: &input.admission.receipt_value,
        request_value: &execution_request,
    })?;
    let mut output_refs = Vec::new();
    if let Some(run) = execution.run.as_ref() {
        output_refs.extend(run.output_refs.iter().cloned());
    }
    Ok(JobExecutionParts {
        request_ref,
        receipt_ref: execution.receipt_ref,
        decision: execution.decision,
        diagnostics: execution.diagnostics,
        output_refs,
    })
}

fn run_gc_workflow(input: GcWorkflowInput<'_>) -> Result<GcRun> {
    let object_ref = dogfood_ref("retention-object")?;
    let requester_ref = dogfood_ref("retention-requester")?;
    let peer_ref = dogfood_ref("retention-peer")?;
    let remote_ref = dogfood_ref("retention-remote-cache")?;
    let remote_refs = vec![remote_ref.clone()];
    let seed = GcSeed {
        root: input.root,
        object_ref: &object_ref,
        requester_ref: &requester_ref,
        peer_ref: &peer_ref,
        remote_ref: &remote_ref,
        remote_refs: &remote_refs,
        object_kind: "chunk",
        class: crate::retention::CLASS_DURABLE_VALUE,
        action: crate::retention::ACTION_DELETE,
    };
    let admissions = gc_admissions(seed)?;
    let flow = gc_flow(input, seed, &admissions.evidence)?;
    let ledger_import_refs = import_gc_values(input.ledger_root, &admissions, &flow)?;
    let (mcp_call, catalog_receipt_ref) = gc_catalog(input.registry_root, input.ledger_root, seed.object_ref)?;
    let bundle_diagnostics = gc_bundle_diagnostics(&flow)?;
    let artifact_refs = gc_artifact_refs(&admissions, &flow, &mcp_call.response_ref, ledger_import_refs);
    Ok(finish_gc_run(GcFinishInput {
        object_ref,
        flow,
        mcp_call,
        catalog_receipt_ref,
        artifact_refs,
        bundle_diagnostics,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GcSeed<'a> {
    root: &'a Path,
    object_ref: &'a str,
    requester_ref: &'a str,
    peer_ref: &'a str,
    remote_ref: &'a str,
    remote_refs: &'a [String],
    object_kind: &'a str,
    class: &'a str,
    action: &'a str,
}

struct GcAdmissions {
    policy: crate::retention::EvidenceAdmission,
    authority: crate::retention::EvidenceAdmission,
    support: crate::retention::EvidenceAdmission,
    index: crate::retention::EvidenceAdmission,
    remote_gc: crate::retention::EvidenceAdmission,
    clearance: crate::retention::RemoteGcClearance,
    evidence: crate::retention::DestructiveEvidence,
}

struct GcFlow {
    plan: crate::retention::GcPlan,
    apply: crate::retention::GcApply,
    execution: crate::retention::GcExecutionGate,
    audit: crate::retention::GcAudit,
    explain: crate::retention::CandidateExplain,
    bundle: crate::retention::CandidateBundle,
    profile: crate::retention::CandidateBundleProfile,
    verify: crate::retention::CandidateBundleVerify,
}

struct GcFinishInput {
    object_ref: String,
    flow: GcFlow,
    mcp_call: crate::catalog_mcp::CatalogMcpCall,
    catalog_receipt_ref: String,
    artifact_refs: Vec<String>,
    bundle_diagnostics: Vec<String>,
}

fn store_gc_fixture(
    seed: GcSeed<'_>,
    kind: &str,
    label: &str,
    remote_refs: &[String],
) -> Result<crate::retention::EvidenceAdmission> {
    store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: seed.root,
        kind,
        label,
        requester_ref: seed.requester_ref,
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        action: seed.action,
        remote_refs,
    })
}

fn gc_admissions(seed: GcSeed<'_>) -> Result<GcAdmissions> {
    let policy = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_POLICY, "policy", &[])?;
    let authority = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_AUTHORITY, "authority", &[])?;
    let support = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", &[])?;
    let index = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_REFERENCE_INDEX, "index", &[])?;
    let remote_gc = store_gc_fixture(seed, crate::retention::ADMISSION_KIND_REMOTE_GC, "remote-gc", seed.remote_refs)?;
    let clearance_evidence = vec![support.admission_ref.clone()];
    let clearance =
        crate::retention::store_remote_gc_clearance(seed.root, &crate::retention::RemoteGcClearanceInput {
            decision: "pass",
            requester_ref: seed.requester_ref,
            peer_ref: seed.peer_ref,
            object_ref: seed.object_ref,
            object_kind: seed.object_kind,
            retention_class: seed.class,
            action: seed.action,
            remote_ref: seed.remote_ref,
            policy_ref: &policy.admission_ref,
            authority_ref: &authority.admission_ref,
            evidence_refs: &clearance_evidence,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })?;
    let evidence = crate::retention::DestructiveEvidence {
        requester_ref: Some(seed.requester_ref.to_string()),
        policy_refs: vec![policy.admission_ref.clone()],
        authority_refs: vec![authority.admission_ref.clone()],
        evidence_refs: vec![support.admission_ref.clone()],
        retained_refs: Vec::new(),
        remote_peer_refs: vec![seed.peer_ref.to_string()],
        remote_refs: seed.remote_refs.to_vec(),
        reference_index_refs: vec![index.admission_ref.clone()],
        remote_gc_refs: vec![remote_gc.admission_ref.clone()],
        remote_clearance_refs: vec![clearance.clearance_ref.clone()],
        is_reference_index_complete: true,
    };
    Ok(GcAdmissions {
        policy,
        authority,
        support,
        index,
        remote_gc,
        clearance,
        evidence,
    })
}

fn gc_flow(
    input: GcWorkflowInput<'_>,
    seed: GcSeed<'_>,
    evidence: &crate::retention::DestructiveEvidence,
) -> Result<GcFlow> {
    let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
        root: input.root,
        subsystem: "ledger-gc",
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        action: seed.action,
        evidence,
    })?;
    let apply = crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
        root: input.root,
        plan_ref: &plan.plan_ref,
    })?;
    let execution = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
        root: input.root,
        subsystem: "ledger-gc",
        action: seed.action,
        object_ref: seed.object_ref,
        object_kind: seed.object_kind,
        retention_class: seed.class,
        apply_ref: Some(&apply.apply_ref),
    })?;
    let audit = crate::retention::audit_gc_execution(crate::retention::GcAuditInput {
        root: input.root,
        execution_ref: &execution.execution_ref,
    })?;
    let explain = crate::retention::explain_candidate(crate::retention::CandidateExplainInput {
        root: input.root,
        object_ref: seed.object_ref,
        object_kind: Some(seed.object_kind),
        retention_class: Some(seed.class),
        action: Some(seed.action),
        subsystem: Some("ledger-gc"),
    })?;
    let bundle = crate::retention::export_candidate_bundle(crate::retention::CandidateBundleExportInput {
        root: input.root,
        explain_value: &explain.value,
        out: input.bundle_dir,
        profile: crate::retention::CandidateBundleExportProfile::Public,
    })?;
    let profile_value = crate::preserves_rail::parse_text(
        &std::fs::read_to_string(input.bundle_dir.join("bundle-profile.preserves")).map_err(MoltenError::from)?,
    )?;
    let profile = crate::retention::parse_candidate_bundle_profile(&profile_value)?;
    let verify = crate::retention::verify_candidate_bundle(crate::retention::CandidateBundleVerifyInput {
        bundle_dir: input.bundle_dir,
    })?;
    Ok(GcFlow {
        plan,
        apply,
        execution,
        audit,
        explain,
        bundle,
        profile,
        verify,
    })
}

fn import_gc_values(root: &Path, admissions: &GcAdmissions, flow: &GcFlow) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for value in [
        &admissions.policy.value,
        &admissions.authority.value,
        &admissions.support.value,
        &admissions.index.value,
        &admissions.remote_gc.value,
        &admissions.clearance.value,
        &flow.plan.value,
        &flow.apply.value,
        &flow.execution.value,
        &flow.audit.value,
        &flow.explain.value,
        &flow.bundle.value,
        &flow.profile.value,
        &flow.verify.value,
    ] {
        let imported = crate::ledger::import_artifact(root, value)?;
        refs.push_limited_value(
            crate::preserves_rail::canonical_hash(&imported.receipt_value)?,
            MAX_OPERATOR_REFS,
            "retention dogfood ledger imports",
        )?;
    }
    Ok(refs)
}

fn gc_catalog(
    registry_root: &Path,
    ledger_root: &Path,
    object_ref: &str,
) -> Result<(crate::catalog_mcp::CatalogMcpCall, String)> {
    let mcp_request = crate::catalog_mcp::mcp_request_value("search_retention_gc", vec![
        crate::preserves_rail::record("stage", vec![crate::preserves_rail::string("audit")]),
        crate::preserves_rail::record("object-ref", vec![crate::preserves_rail::string(object_ref)]),
        crate::preserves_rail::record("subsystem", vec![crate::preserves_rail::string("ledger-gc")]),
    ])?;
    let mcp_call = crate::catalog_mcp::call(registry_root, Some(ledger_root), &mcp_request)?;
    let catalog_receipt_ref = crate::preserves_rail::canonical_hash(&mcp_call.receipt_value)?;
    Ok((mcp_call, catalog_receipt_ref))
}

fn gc_bundle_diagnostics(flow: &GcFlow) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle", &flow.bundle.diagnostics)?;
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle-profile", &flow.profile.diagnostics)?;
    append_dogfood_diagnostics(&mut diagnostics, "retention-bundle-verify", &flow.verify.diagnostics)?;
    Ok(diagnostics)
}

fn gc_artifact_refs(
    admissions: &GcAdmissions,
    flow: &GcFlow,
    response_ref: &str,
    ledger_import_refs: Vec<String>,
) -> Vec<String> {
    let mut refs = vec![
        admissions.policy.admission_ref.clone(),
        admissions.authority.admission_ref.clone(),
        admissions.support.admission_ref.clone(),
        admissions.index.admission_ref.clone(),
        admissions.remote_gc.admission_ref.clone(),
        admissions.clearance.clearance_ref.clone(),
        flow.plan.plan_ref.clone(),
        flow.apply.apply_ref.clone(),
        flow.execution.execution_ref.clone(),
        flow.audit.audit_ref.clone(),
        flow.explain.explain_ref.clone(),
        flow.bundle.bundle_ref.clone(),
        flow.profile.profile_ref.clone(),
        flow.verify.verify_ref.clone(),
        response_ref.to_string(),
    ];
    refs.extend(ledger_import_refs);
    refs
}

fn finish_gc_run(input: GcFinishInput) -> GcRun {
    let GcFinishInput {
        object_ref,
        flow,
        mcp_call,
        catalog_receipt_ref,
        artifact_refs,
        bundle_diagnostics,
    } = input;
    let GcFlow {
        plan,
        apply,
        execution,
        audit,
        explain,
        bundle,
        profile,
        verify,
    } = flow;
    GcRun {
        object_ref,
        plan_ref: plan.plan_ref,
        plan_decision: plan.decision,
        plan_diagnostics: plan.diagnostics,
        apply_ref: apply.apply_ref,
        apply_decision: apply.decision,
        apply_diagnostics: apply.diagnostics,
        execution_ref: execution.execution_ref,
        execution_decision: execution.decision,
        execution_diagnostics: execution.diagnostics,
        audit_ref: audit.audit_ref,
        audit_decision: audit.decision,
        audit_diagnostics: audit.diagnostics,
        explain_ref: explain.explain_ref,
        bundle_ref: bundle.bundle_ref,
        bundle_profile_ref: profile.profile_ref,
        bundle_verify_ref: verify.verify_ref,
        bundle_verify_decision: verify.decision,
        bundle_diagnostics,
        catalog_request_ref: mcp_call.request.request_ref,
        catalog_receipt_ref,
        catalog_response_ref: mcp_call.response_ref,
        catalog_decision: mcp_call.decision,
        artifact_refs,
    }
}

fn store_retention_admission_fixture(
    input: RetentionAdmissionFixtureInput<'_>,
) -> Result<crate::retention::EvidenceAdmission> {
    let bound_refs = vec![dogfood_ref(&format!("retention-{}-bound", input.label))?];
    crate::retention::store_evidence_admission(input.root, &crate::retention::EvidenceAdmissionInput {
        kind: input.kind,
        decision: "pass",
        requester_ref: input.requester_ref,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
        bound_refs: &bound_refs,
        retained_refs: &[],
        remote_refs: input.remote_refs,
        is_reference_index_complete: true,
        is_current: true,
        revoked_refs: &[],
        diagnostics: &[],
    })
}

fn append_dogfood_diagnostics(sink: &mut impl PushLimited<String>, label: &str, diagnostics: &[String]) -> Result<()> {
    for diagnostic in diagnostics {
        sink.push_limited_value(
            format!("{label}:{diagnostic}"),
            MAX_OPERATOR_DIAGNOSTICS,
            "operator dogfood diagnostics",
        )?;
    }
    Ok(())
}

fn install_job_execute_authority_context(
    registry: &Path,
    job_ref: &str,
    policy_refs: &[String],
    capability_refs: &[String],
) -> Result<String> {
    let subject_ref = dogfood_ref("target-peer-subject")?;
    let context_value = crate::authority::authority_context_value(crate::authority::ContextValueInput {
        subject_ref: &subject_ref,
        capabilities: &[crate::authority::AuthorityCapability {
            capability: "job:execute".to_string(),
            scope: job_ref.to_string(),
            attenuation: "scoped".to_string(),
        }],
        delegation_refs: &[],
        not_before: None,
        expires_at: None,
        revocation_refs: &[],
        key_refs: &[],
        policy_refs,
        evidence_refs: &[dogfood_ref("authority-evidence")?],
    })?;
    let context_ref = crate::preserves_rail::canonical_hash(&context_value)?;
    crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "authority-context".to_string(),
        payload: context_value,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("authority-evidence")?],
        installer_ref: dogfood_ref("authority-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(context_ref)
}

fn install_clean_octet_gate(registry: &Path, policy_refs: &[String], capability_refs: &[String]) -> Result<String> {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let gate_ref = crate::preserves_rail::canonical_hash(&gate_value)?;
    crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "octet-gate-receipt".to_string(),
        payload: gate_value,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("octet-evidence")?],
        installer_ref: dogfood_ref("octet-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    Ok(gate_ref)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DogfoodRepro {
    report_ref: String,
    gate_ref: String,
    bundle_ref: String,
    verify_ref: String,
}

fn build_dogfood_repro() -> Result<DogfoodRepro> {
    let suite = crate::preserves_rail::parse_text(DOGFOOD_HARNESS_SUITE)?;
    let run = crate::harness::run_suite_value(&suite)?;
    let gate = crate::harness::gate_receipt_value(&crate::harness::gate_check_value(&run.report_value)?);
    let gate_ref = crate::preserves_rail::canonical_hash(&gate)?;
    let bundle = crate::harness::sealed_repro_bundle_value_with_command(&run.report_value, &[
        "molten".to_string(),
        "dogfood".to_string(),
        "local-node".to_string(),
    ])?;
    let bundle_ref = crate::preserves_rail::canonical_hash(&bundle)?;
    let verify = crate::harness::repro_verify_receipt_value(&bundle)?;
    let verify_ref = crate::preserves_rail::canonical_hash(&verify)?;
    Ok(DogfoodRepro {
        report_ref: run.report_ref,
        gate_ref,
        bundle_ref,
        verify_ref,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DogfoodEvidenceImportInput<'a> {
    ledger_root: &'a Path,
    workflow_value: &'a IoValue,
    step_values: &'a [IoValue],
    checkpoint_values: &'a [IoValue],
    report_value: &'a IoValue,
    release_gate_value: Option<&'a IoValue>,
    replay_verify_value: &'a IoValue,
    replay_index_value: &'a IoValue,
}

fn import_dogfood_evidence(input: DogfoodEvidenceImportInput<'_>) -> Result<Vec<String>> {
    let DogfoodEvidenceImportInput {
        ledger_root,
        workflow_value,
        step_values,
        checkpoint_values,
        report_value,
        release_gate_value,
        replay_verify_value,
        replay_index_value,
    } = input;
    let mut imports = Vec::new();
    for value in step_values
        .iter()
        .chain(checkpoint_values.iter())
        .chain(std::iter::once(workflow_value))
        .chain(std::iter::once(report_value))
        .chain(std::iter::once(replay_verify_value))
        .chain(std::iter::once(replay_index_value))
        .chain(release_gate_value)
    {
        let import = crate::ledger::import_artifact(ledger_root, value)?;
        imports.push_limited_value(
            crate::preserves_rail::canonical_hash(&import.receipt_value)?,
            MAX_OPERATOR_REFS,
            "dogfood ledger import refs",
        )?;
    }
    Ok(imports)
}

fn service_lifecycle_pass(value: &IoValue) -> bool {
    crate::service_records::parse_service_lifecycle_receipt(value).is_ok_and(|receipt| receipt.decision == "pass")
}

fn dirty_state_reason(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_dir() {
        return Ok(Some("dogfood state root exists but is not a directory".to_string()));
    }
    let mut entries = std::fs::read_dir(path).map_err(MoltenError::from)?;
    if entries.next().transpose().map_err(MoltenError::from)?.is_some() {
        Ok(Some("dogfood local-node requires a clean empty state root".to_string()))
    } else {
        Ok(None)
    }
}

fn dogfood_ref(label: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("operator-dogfood-ref", vec![
        crate::preserves_rail::string(label),
    ]))
}

fn validate_workflow_id(value: &str) -> Result<()> {
    validate_non_empty(value, "operator workflow id")?;
    if !value.starts_with("dogfood:") {
        return Err(MoltenError::invalid_harness(format!("operator workflow id {value} must start with dogfood:")));
    }
    Ok(())
}

fn validate_step_name(value: &str) -> Result<()> {
    validate_non_empty(value, "operator step name")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(MoltenError::invalid_harness(format!("unsupported operator step name {value}")));
    }
    Ok(())
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        "pass" | "deny" | "diagnostic" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported operator decision {value}"))),
    }
}

fn validate_replay_status(value: &str) -> Result<()> {
    match value {
        "deterministic" | "recorded" | "diagnostic" | "non-replayable" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported operator replay status {value}"))),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}")))
}

fn validate_optional_ref(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, field)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], field: &str) -> Result<()> {
    for value in values {
        validate_ref(value, field)?;
    }
    Ok(())
}

fn require_non_empty_refs(values: &[String], field: &str) -> Result<()> {
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_refs(values, field)
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds {maximum}")))
    } else {
        Ok(())
    }
}

trait PushLimited<T> {
    fn push_limited_value(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T> PushLimited<T> for Vec<T> {
    fn push_limited_value(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.len().saturating_add(1), maximum, label)?;
        self.push(value);
        Ok(())
    }
}

fn status(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn step_receipts_sequence(receipts: &[(String, String)]) -> IoValue {
    crate::preserves_rail::sequence(
        receipts
            .iter()
            .map(|(name, reference)| {
                crate::preserves_rail::record("step", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(reference),
                ])
            })
            .collect(),
    )
}

fn file_refs_sequence(refs: &[(String, String)]) -> IoValue {
    crate::preserves_rail::sequence(
        refs.iter()
            .map(|(name, reference)| {
                crate::preserves_rail::record("file", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(reference),
                ])
            })
            .collect(),
    )
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "operator checks")?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, "operator checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = crate::preserves_rail::value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "operator check name")?;
        let status = required_string(&check[1], "operator check status")?;
        if status != "pass" && status != "fail" && status != "diagnostic" {
            return Err(MoltenError::invalid_harness(format!("operator check {name} has status {status}")));
        }
        parsed.push_limited_value((name, status), MAX_OPERATOR_REFS, "operator checks")?;
    }
    Ok(parsed)
}

fn workflow_check_pass(checks: &[(String, String)], expected: &str) -> bool {
    checks.iter().any(|(name, status)| name == expected && status == "pass")
}

fn require_check(checks: &[(String, String)], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(name, _)| name == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_bool(value: &Value<IoValue>, label: &str) -> Result<bool> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let number = fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {label}")))
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_ref(item.as_ref(), label))
        .collect()
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_string(item.as_ref(), label))
        .collect()
}

fn record_iovalue_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited_value(crate::preserves_rail::value_to_iovalue(item), MAX_OPERATOR_REFS, label)?;
    }
    Ok(values)
}

fn record_step_receipts(value: &Value<IoValue>, label: &str) -> Result<Vec<(String, String)>> {
    let items = record_iovalue_sequence(value, label)?;
    let mut receipts = Vec::new();
    for item in &items {
        let fields = simple_record(item, "step", 2)?;
        let name = required_string(&fields[0], "dogfood step receipt name")?;
        let reference = required_ref(&fields[1], "dogfood step receipt ref")?;
        receipts.push_limited_value((name, reference), MAX_OPERATOR_REFS, "dogfood step receipts")?;
    }
    Ok(receipts)
}

fn record_file_refs(value: &Value<IoValue>, label: &str) -> Result<Vec<(String, String)>> {
    let items = record_iovalue_sequence(value, label)?;
    let mut files = Vec::new();
    for item in &items {
        let fields = simple_record(item, "file", 2)?;
        let name = required_string(&fields[0], "Nix dogfood file name")?;
        let reference = required_ref(&fields[1], "Nix dogfood file ref")?;
        files.push_limited_value((name, reference), MAX_OPERATOR_REFS, "Nix dogfood file refs")?;
    }
    Ok(files)
}

fn member_ref(value: &Value<IoValue>, expected_name: &str) -> Result<String> {
    record_file_refs(value, "members")?
        .into_iter()
        .find_map(|(name, reference)| (name == expected_name).then_some(reference))
        .ok_or_else(|| MoltenError::invalid_harness(format!("release evidence bundle missing member {expected_name}")))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| MoltenError::invalid_harness(format!("{field} overflows u64: {error}")))
}

#[cfg(test)]
mod tests {
    type PathBuf = std::path::PathBuf;
    type SignReceiptInput<'a> = crate::evidence::SignReceiptInput<'a>;
    type SignedReceiptKeyInput<'a> = crate::evidence::SignedReceiptKeyInput<'a>;
    type SignedReceiptKeyRevocationInput<'a> = crate::evidence::SignedReceiptKeyRevocationInput<'a>;

    use super::*;

    fn parse_signed_receipt_key(value: &IoValue) -> Result<SignedReceiptKey> {
        crate::evidence::parse_signed_receipt_key(value)
    }

    fn parse_signed_receipt_key_revocation(value: &IoValue) -> Result<SignedReceiptKeyRevocation> {
        crate::evidence::parse_signed_receipt_key_revocation(value)
    }

    fn sign_receipt(input: &SignReceiptInput<'_>) -> Result<IoValue> {
        crate::evidence::sign_receipt(input)
    }

    fn signed_receipt_key_value(input: &SignedReceiptKeyInput<'_>) -> Result<IoValue> {
        crate::evidence::signed_receipt_key_value(input)
    }

    fn signed_receipt_key_revocation_value(input: &SignedReceiptKeyRevocationInput<'_>) -> Result<IoValue> {
        crate::evidence::signed_receipt_key_revocation_value(input)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn local_node_dogfood_runs_and_gates_release() {
        let root = temp_dir("operator-dogfood-pass");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dogfood run");
        assert_eq!(run.decision, "pass", "{}", to_text(&run.report_value).expect("report text"));
        let release_gate_ref = run.release_gate_ref.as_deref().expect("release gate ref");
        crate::preserves_rail::validate_content_ref(release_gate_ref).expect("release gate ref is canonical");
        assert_eq!(crate::ledger::artifact_kind(&run.workflow_value), "operator-workflow");
        assert_eq!(crate::ledger::artifact_kind(&run.report_value), "dogfood-report");
        assert_eq!(
            crate::ledger::artifact_kind(run.release_gate_value.as_ref().expect("release gate")),
            "release-gate-receipt"
        );
        let entries = crate::ledger::list_artifacts(&root.join("ledger")).expect("ledger entries");
        assert!(entries.iter().any(|entry| entry.artifact_kind == "dogfood-report"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "operator-checkpoint"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "retention-gc-audit"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "retention-candidate-bundle-verify"));
        let workflow = parse_operator_workflow(&run.workflow_value).expect("parse workflow");
        assert!(workflow.steps.iter().any(|step| step.name == "plan-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "apply-retention-gc-plan"));
        assert!(workflow.steps.iter().any(|step| step.name == "execute-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "audit-retention-gc"));
        assert!(workflow.steps.iter().any(|step| step.name == "export-retention-gc-bundle"));
        assert!(workflow.steps.iter().any(|step| step.name == "search-retention-gc-catalog"));
        assert!(workflow.steps.iter().any(|step| step.name == "index-replay-evidence"));
        assert_eq!(
            crate::ledger::artifact_kind(run.replay_index_value.as_ref().expect("replay index")),
            "deterministic-replay-index"
        );
        let release_text = to_text(run.release_gate_value.as_ref().expect("release gate text")).expect("release text");
        assert!(release_text.contains("replay-evidence-index-bound"));
        assert!(release_text.contains("replay-index-is-evidence-only"));
        assert!(release_text.contains("retention-gc-review-bound"));
        assert!(release_text.contains("retention-gc-is-evidence-only"));
        assert!(operator_dogfood_summary(&run.report_value).expect("summary").contains("decision=pass"));
    }

    #[test]
    fn nix_dogfood_release_evidence_verifies_and_denies_stale_refs() {
        let case = build_nix_case();
        assert_release_binding_search(&case);
        let signed_members = signed_members(&case);
        let signed_bundle_verify = signed_bundle_pass(&case, &signed_members);
        let key = signed_key();
        assert_promotion_receipts(&case, &signed_bundle_verify, &key);
        assert_signed_denials(&case, &signed_members, &key);
        assert_stale_bundle_denies(&case);
        assert_stale_evidence_denies(&case);
    }

    struct NixCase {
        root: PathBuf,
        output_root: PathBuf,
        run: LocalNodeDogfoodRun,
        parsed: NixDogfoodEvidence,
        evidence: IoValue,
        receipt: NixDogfoodVerifyReceipt,
        bundle: IoValue,
        parsed_bundle: ReleaseEvidenceBundle,
        bundle_verify: ReleaseEvidenceBundleVerifyReceipt,
    }

    struct PromotionInput<'a> {
        output_path: &'a std::path::Path,
        bundle_verify_value: &'a IoValue,
        source_evidence: &'a str,
        key: &'a SignedReceiptKey,
        revocations: &'a [SignedReceiptKeyRevocation],
    }

    fn build_nix_case() -> NixCase {
        let root = temp_dir("operator-dogfood-nix-evidence");
        let state_root = root.join("state");
        let output_root = root.join("nix-output");
        std::fs::create_dir_all(&output_root).expect("create nix output");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput {
            state_root: &state_root,
        })
        .expect("dogfood run");
        write_run_outputs(&output_root, &run);
        let evidence = nix_dogfood_release_evidence_value(&NixDogfoodEvidenceInput {
            output_path: &output_root,
        })
        .expect("nix evidence");
        let parsed = parse_nix_dogfood_evidence(&evidence).expect("parse nix evidence");
        assert_eq!(crate::ledger::artifact_kind(&evidence), "nix-dogfood-release-evidence");
        assert_eq!(parsed.release_gate_ref, run.release_gate_ref.clone().expect("release ref"));
        assert_eq!(parsed.replay_verify_ref, run.replay_verify_ref.clone().expect("replay verify ref"));
        assert_eq!(parsed.replay_index_ref, run.replay_index_ref.clone().expect("replay index ref"));
        let receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &output_root,
            evidence_value: &evidence,
        })
        .expect("verify nix evidence");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&receipt.value), "nix-dogfood-release-verify-receipt");
        assert_tampered_replay_denies(&output_root, &run, &evidence);
        write_bundle_inputs(&output_root, &evidence, &receipt);
        let bundle = release_evidence_bundle_value(&ReleaseEvidenceBundleInput {
            output_path: &output_root,
        })
        .expect("release bundle");
        let parsed_bundle = parse_release_evidence_bundle(&bundle).expect("parse release bundle");
        assert_eq!(crate::ledger::artifact_kind(&bundle), "release-evidence-bundle");
        assert_eq!(parsed_bundle.report_ref, parsed.report_ref);
        assert_eq!(parsed_bundle.replay_verify_ref, parsed.replay_verify_ref);
        assert_eq!(parsed_bundle.replay_index_ref, parsed.replay_index_ref);
        let bundle_verify = unsigned_bundle_verify(&output_root, &bundle);
        assert_eq!(bundle_verify.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&bundle_verify.value), "release-evidence-bundle-verify-receipt");
        NixCase {
            root,
            output_root,
            run,
            parsed,
            evidence,
            receipt,
            bundle,
            parsed_bundle,
            bundle_verify,
        }
    }

    fn write_run_outputs(output_root: &std::path::Path, run: &LocalNodeDogfoodRun) {
        std::fs::write(output_root.join("dogfood-report.preserves"), to_text(&run.report_value).expect("report text"))
            .expect("write report");
        std::fs::write(
            output_root.join("release-gate.preserves"),
            to_text(run.release_gate_value.as_ref().expect("release gate")).expect("release text"),
        )
        .expect("write release gate");
        std::fs::write(
            output_root.join("replay-verify.preserves"),
            to_text(run.replay_verify_value.as_ref().expect("replay verify")).expect("replay verify text"),
        )
        .expect("write replay verify");
        std::fs::write(
            output_root.join("replay-evidence-index.preserves"),
            to_text(run.replay_index_value.as_ref().expect("replay index")).expect("replay index text"),
        )
        .expect("write replay index");
        std::fs::write(
            output_root.join("dogfood-summary.txt"),
            format!(
                "dogfood local-node decision=pass report={} release-gate={}\n",
                run.report_ref,
                run.release_gate_ref.as_deref().expect("release ref")
            ),
        )
        .expect("write summary");
        std::fs::write(output_root.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n")
            .expect("write nextest marker");
    }

    fn assert_tampered_replay_denies(output_root: &std::path::Path, run: &LocalNodeDogfoodRun, evidence: &IoValue) {
        std::fs::write(output_root.join("replay-evidence-index.preserves"), "<tampered-replay-index>\n")
            .expect("tamper replay index");
        let tampered_replay_verify = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: output_root,
            evidence_value: evidence,
        })
        .expect("verify tampered replay index evidence");
        assert_eq!(tampered_replay_verify.decision, "deny");
        assert!(
            tampered_replay_verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("replay index") || diagnostic.contains("observation failed"))
        );
        std::fs::write(
            output_root.join("replay-evidence-index.preserves"),
            to_text(run.replay_index_value.as_ref().expect("replay index")).expect("replay index text"),
        )
        .expect("restore replay index");
    }

    fn write_bundle_inputs(output_root: &std::path::Path, evidence: &IoValue, receipt: &NixDogfoodVerifyReceipt) {
        std::fs::write(output_root.join("nix-dogfood-evidence.preserves"), to_text(evidence).expect("evidence text"))
            .expect("write evidence");
        std::fs::write(output_root.join("nix-dogfood-verify.preserves"), to_text(&receipt.value).expect("verify text"))
            .expect("write verify");
    }

    fn unsigned_bundle_verify(output_root: &std::path::Path, bundle: &IoValue) -> ReleaseEvidenceBundleVerifyReceipt {
        verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: output_root,
            bundle_value: bundle,
            signed_member_values: &[],
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "local-release-trust-root",
            signed_key: "local-release-key",
            signed_keys: &[],
            signed_key_revocations: &[],
            signed_key_ref: None,
            signed_key_id: None,
            signed_signer: None,
            is_signed_members_required: false,
        })
        .expect("verify release bundle")
    }

    fn assert_release_binding_search(case: &NixCase) {
        let catalog_registry = case.root.join("catalog-registry");
        let release_ledger = case.root.join("release-ledger");
        crate::ledger::import_artifact(&release_ledger, case.run.release_gate_value.as_ref().expect("release gate"))
            .expect("import release gate");
        crate::ledger::import_artifact(&release_ledger, case.run.replay_verify_value.as_ref().expect("replay verify"))
            .expect("import replay verify");
        crate::ledger::import_artifact(&release_ledger, case.run.replay_index_value.as_ref().expect("replay index"))
            .expect("import replay index");
        crate::ledger::import_artifact(&release_ledger, &case.evidence).expect("import Nix evidence");
        crate::ledger::import_artifact(&release_ledger, &case.bundle_verify.value).expect("import bundle verify");
        let replay_binding_request = crate::catalog_mcp::mcp_request_value("search_replay_evidence", vec![
            crate::preserves_rail::record("stage", vec![crate::preserves_rail::string("release-binding")]),
            crate::preserves_rail::record("release-replay-index-ref", vec![crate::preserves_rail::string(
                &case.parsed.replay_index_ref,
            )]),
        ])
        .expect("replay binding request");
        let replay_binding =
            crate::catalog_mcp::call(&catalog_registry, Some(&release_ledger), &replay_binding_request)
                .expect("replay binding search");
        assert_eq!(replay_binding.decision, "pass");
        assert!(
            to_text(&replay_binding.response_value)
                .expect("replay binding response")
                .contains("deterministic-replay:release-binding")
        );
    }

    fn signed_members(case: &NixCase) -> Vec<IoValue> {
        vec![
            sign_member(&case.run.report_value),
            sign_member(case.run.release_gate_value.as_ref().expect("release gate")),
            sign_member(case.run.replay_verify_value.as_ref().expect("replay verify")),
            sign_member(case.run.replay_index_value.as_ref().expect("replay index")),
            sign_member(&case.evidence),
            sign_member(&case.receipt.value),
        ]
    }

    fn sign_member(receipt: &IoValue) -> IoValue {
        sign_receipt(&SignReceiptInput {
            receipt,
            signer: "release-signer",
            purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            trust_root: "release-root",
            key: "release-key",
            parents: &[],
        })
        .expect("sign member")
    }

    fn signed_bundle_pass(case: &NixCase, signed_members: &[IoValue]) -> ReleaseEvidenceBundleVerifyReceipt {
        let signed_bundle_verify = required_bundle_verify(case, signed_members, Some("release-signer"), None);
        assert_eq!(signed_bundle_verify.decision, "pass");
        signed_bundle_verify
    }

    fn signed_key() -> SignedReceiptKey {
        let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
            key_id: "release-key-1",
            signer: "release-signer",
            trust_root: "release-root",
            key: "release-key",
            generation: 1,
            predecessor_ref: None,
        })
        .expect("signed key value");
        parse_signed_receipt_key(&key_value).expect("parse signed key")
    }

    fn assert_promotion_receipts(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(promotion.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&promotion.value), "release-promotion-gate-receipt");
        let revocation = signed_revocation(key);
        assert_revoked_promotion(case, signed_bundle_verify, key, &revocation);
        assert_missing_source_promotion(case, signed_bundle_verify, key);
        assert_stale_output_promotion(case, signed_bundle_verify, key);
    }

    fn signed_revocation(key: &SignedReceiptKey) -> SignedReceiptKeyRevocation {
        let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
            key,
            reason: "test-revoked",
            superseded_by: None,
        })
        .expect("revocation value");
        parse_signed_receipt_key_revocation(&revocation_value).expect("parse revocation")
    }

    fn assert_revoked_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
        revocation: &SignedReceiptKeyRevocation,
    ) {
        let revoked_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: std::slice::from_ref(revocation),
        });
        assert_eq!(revoked_promotion.decision, "deny");
        assert!(
            revoked_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("revoked") || diagnostic.contains("stale"))
        );
    }

    fn assert_missing_source_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let missing_source_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "",
            key,
            revocations: &[],
        });
        assert_eq!(missing_source_promotion.decision, "deny");
        assert!(missing_source_promotion.diagnostics.iter().any(|diagnostic| diagnostic.contains("source evidence")));
    }

    fn assert_stale_output_promotion(
        case: &NixCase,
        signed_bundle_verify: &ReleaseEvidenceBundleVerifyReceipt,
        key: &SignedReceiptKey,
    ) {
        let stale_output = case.output_root.join("stale-output");
        let stale_output_promotion = promotion_receipt(PromotionInput {
            output_path: &stale_output,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(stale_output_promotion.decision, "deny");
        assert!(
            stale_output_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("output-path-ref mismatch"))
        );
    }

    fn promotion_receipt(input: PromotionInput<'_>) -> ReleasePromotionGateReceipt {
        release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: input.output_path,
            bundle_verify_value: input.bundle_verify_value,
            source_evidence: input.source_evidence,
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(input.key),
            signed_key_revocations: input.revocations,
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&input.key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("promotion receipt")
    }

    fn assert_signed_denials(case: &NixCase, signed_members: &[IoValue], key: &SignedReceiptKey) {
        let missing_signed_member_verify = required_bundle_verify(case, &[], Some("release-signer"), Some(key));
        assert_eq!(missing_signed_member_verify.decision, "deny");
        let denied_bundle_promotion = promotion_receipt(PromotionInput {
            output_path: &case.output_root,
            bundle_verify_value: &missing_signed_member_verify.value,
            source_evidence: "source:working-tree-reviewed",
            key,
            revocations: &[],
        });
        assert_eq!(denied_bundle_promotion.decision, "deny");
        assert!(denied_bundle_promotion.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));
        let wrong_signer_verify = required_bundle_verify(case, signed_members, Some("wrong-signer"), None);
        assert_eq!(wrong_signer_verify.decision, "deny");
        assert!(wrong_signer_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("signer")));
        let missing_signed_verify = required_bundle_verify(case, &signed_members[..1], Some("release-signer"), None);
        assert_eq!(missing_signed_verify.decision, "deny");
        assert!(
            missing_signed_verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing signed member receipt"))
        );
    }

    fn required_bundle_verify(
        case: &NixCase,
        signed_member_values: &[IoValue],
        signed_signer: Option<&str>,
        key: Option<&SignedReceiptKey>,
    ) -> ReleaseEvidenceBundleVerifyReceipt {
        let empty_keys: &[SignedReceiptKey] = &[];
        let signed_keys = key.map(std::slice::from_ref).unwrap_or(empty_keys);
        verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &case.output_root,
            bundle_value: &case.bundle,
            signed_member_values,
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys,
            signed_key_revocations: &[],
            signed_key_ref: key.map(|key| key.key_ref.as_str()),
            signed_key_id: key.map(|_| "release-key-1"),
            signed_signer,
            is_signed_members_required: true,
        })
        .expect("verify required release bundle")
    }

    fn assert_stale_bundle_denies(case: &NixCase) {
        let stale_bundle_ref = dogfood_ref("stale-bundle-summary").expect("stale bundle ref");
        let stale_bundle_text = to_text(&case.bundle)
            .expect("bundle text")
            .replace(&case.parsed_bundle.summary_ref, &stale_bundle_ref);
        let stale_bundle = crate::preserves_rail::parse_text(&stale_bundle_text).expect("stale bundle parse");
        let stale_bundle_verify = unsigned_bundle_verify(&case.output_root, &stale_bundle);
        assert_eq!(stale_bundle_verify.decision, "deny");
        assert!(stale_bundle_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
    }

    fn assert_stale_evidence_denies(case: &NixCase) {
        let stale_ref = dogfood_ref("stale-summary").expect("stale ref");
        let stale_text = to_text(&case.evidence).expect("evidence text").replace(&case.parsed.summary_ref, &stale_ref);
        let stale_evidence = crate::preserves_rail::parse_text(&stale_text).expect("stale evidence parse");
        let stale_receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &case.output_root,
            evidence_value: &stale_evidence,
        })
        .expect("verify stale evidence");
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
    }

    #[test]
    fn missing_receipt_and_non_replayable_mandatory_steps_deny_report() {
        let report = report_with_mandatory_gaps();
        let parsed = parse_dogfood_report(&report).expect("parse report");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("lacks canonical receipt")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-release replay status")));
        assert_gate_rejects(&report);
    }

    fn report_with_mandatory_gaps() -> IoValue {
        let request_ref = dogfood_ref("request").expect("request ref");
        let missing_step = mandatory_step("install-artifact", &request_ref, None, "deterministic");
        let live_receipt = dogfood_ref("live-receipt").expect("live receipt");
        let live_step = mandatory_step("live-diagnostic", &request_ref, Some(&live_receipt), "non-replayable");
        let policy_refs = vec![dogfood_ref("policy").expect("policy")];
        let capability_refs = vec![dogfood_ref("capability").expect("capability")];
        let resource_refs = vec![dogfood_ref("resource").expect("resource")];
        let workflow = operator_workflow_value(&OperatorWorkflowInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            steps: &[missing_step, live_step],
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            replay_profile: "recorded",
        })
        .expect("workflow");
        let checkpoint = operator_checkpoint_value(&OperatorCheckpointInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            sequence: 0,
            step_ref: &dogfood_ref("step").expect("step"),
            request_ref: Some(&request_ref),
            receipt_ref: None,
            result_ref: None,
            state_root_ref: &dogfood_ref("state").expect("state"),
        })
        .expect("checkpoint");
        dogfood_report_value(&DogfoodReportInput {
            workflow_value: &workflow,
            checkpoint_values: &[checkpoint],
            gate_receipt_refs: &[dogfood_ref("gate").expect("gate")],
            repro_bundle_refs: &[dogfood_ref("repro").expect("repro")],
            final_state_ref: &dogfood_ref("final-state").expect("final state"),
            diagnostics: &[],
        })
        .expect("report")
    }

    fn mandatory_step(name: &str, request_ref: &str, receipt_ref: Option<&str>, replay_status: &str) -> IoValue {
        operator_step_value(&OperatorStepInput {
            name,
            request_ref: Some(request_ref),
            receipt_ref,
            decision: "pass",
            replay_status,
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("mandatory step")
    }

    fn assert_gate_rejects(report: &IoValue) {
        assert!(
            release_gate_receipt_value(&ReleaseGateInput {
                report_value: report,
                node_startup_ref: &dogfood_ref("startup").expect("startup"),
                node_shutdown_ref: &dogfood_ref("shutdown").expect("shutdown"),
                harness_gate_refs: &[dogfood_ref("harness-gate").expect("harness gate")],
                catalog_query_refs: &[dogfood_ref("catalog").expect("catalog")],
                repro_verify_refs: &[dogfood_ref("verify").expect("verify")],
                replay_index_refs: &[dogfood_ref("replay-index").expect("replay index")],
                gc_refs: &[dogfood_ref("retention-gc").expect("retention gc")],
                validation_command_refs: &[dogfood_ref("validation").expect("validation")],
            })
            .is_err()
        );
    }

    #[test]
    fn missing_redaction_or_stale_operator_policy_denies_report() {
        let request_ref = dogfood_ref("request").expect("request ref");
        let receipt_ref = dogfood_ref("receipt").expect("receipt ref");
        let step = operator_step_value(&OperatorStepInput {
            name: "gate-evidence",
            request_ref: Some(&request_ref),
            receipt_ref: Some(&receipt_ref),
            decision: "pass",
            replay_status: "deterministic",
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("step");
        let workflow = operator_workflow_value(&OperatorWorkflowInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            steps: &[step],
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[dogfood_ref("resource").expect("resource")],
            replay_profile: "deterministic",
        })
        .expect("workflow");
        let checkpoint = operator_checkpoint_value(&OperatorCheckpointInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            sequence: 0,
            step_ref: &dogfood_ref("step").expect("step"),
            request_ref: Some(&request_ref),
            receipt_ref: Some(&receipt_ref),
            result_ref: Some(&receipt_ref),
            state_root_ref: &dogfood_ref("state").expect("state"),
        })
        .expect("checkpoint");
        let report = dogfood_report_value(&DogfoodReportInput {
            workflow_value: &workflow,
            checkpoint_values: &[checkpoint],
            gate_receipt_refs: &[dogfood_ref("gate").expect("gate")],
            repro_bundle_refs: &[],
            final_state_ref: &dogfood_ref("final-state").expect("final state"),
            diagnostics: &[],
        })
        .expect("report");
        let parsed = parse_dogfood_report(&report).expect("parse report");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("operator policy/capability")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("sealed/redacted repro")));
    }

    #[test]
    fn dirty_state_root_denies_without_release_gate() {
        let root = temp_dir("operator-dogfood-dirty");
        std::fs::write(root.join("leftover"), "dirty").expect("write dirty marker");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dirty report");
        assert_eq!(run.decision, "deny");
        assert!(run.release_gate_value.is_none());
        let report = parse_dogfood_report(&run.report_value).expect("parse dirty report");
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.contains("clean empty state root")));
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
