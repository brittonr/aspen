//! Operator dogfood workflow records and local-node runner.
//!
//! Text rendered by `operator_dogfood_summary` is a non-normative view. The
//! canonical pass/fail evidence is the Preserves report, checkpoint, and gate
//! receipt graph emitted by this module.

use std::fs;
use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::artifacts;
use crate::authority;
use crate::catalog_mcp;
use crate::error::MoltenError;
use crate::error::Result;
use crate::evidence::SignedReceiptKey;
use crate::evidence::SignedReceiptKeyRevocation;
use crate::evidence::VerifySignedReceiptKeyringPolicy;
use crate::evidence::VerifySignedReceiptPolicy;
use crate::evidence::verify_signed_receipt_with_keyring_policy;
use crate::evidence::verify_signed_receipt_with_policy;
use crate::harness;
use crate::job_dag;
use crate::ledger;
use crate::node_identity;
use crate::node_runtime;
use crate::octet_gate;
use crate::preserves_rail::OPERATOR_CHECKPOINT_SCHEMA;
use crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA;
use crate::preserves_rail::OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA;
use crate::preserves_rail::OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA;
use crate::preserves_rail::OPERATOR_STEP_SCHEMA;
use crate::preserves_rail::OPERATOR_WORKFLOW_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;
use crate::remote_dataspace;
use crate::retention;

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
    pub steps: &'a [IOValue],
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub replay_profile: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogfoodReportInput<'a> {
    pub workflow_value: &'a IOValue,
    pub checkpoint_values: &'a [IOValue],
    pub gate_receipt_refs: &'a [String],
    pub repro_bundle_refs: &'a [String],
    pub final_state_ref: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateInput<'a> {
    pub report_value: &'a IOValue,
    pub node_startup_ref: &'a str,
    pub node_shutdown_ref: &'a str,
    pub harness_gate_refs: &'a [String],
    pub catalog_query_refs: &'a [String],
    pub repro_verify_refs: &'a [String],
    pub retention_gc_refs: &'a [String],
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
    pub retention_gc_refs: Vec<String>,
    pub validation_command_refs: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
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
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub file_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyInput<'a> {
    pub output_path: &'a Path,
    pub evidence_value: &'a IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub evidence_ref: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
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
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub member_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleVerifyInput<'a> {
    pub output_path: &'a Path,
    pub bundle_value: &'a IOValue,
    pub signed_member_values: &'a [IOValue],
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
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionGateInput<'a> {
    pub output_path: &'a Path,
    pub bundle_verify_value: &'a IOValue,
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
    pub value: IOValue,
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
    pub value: IOValue,
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportVerifyInput<'a> {
    pub manifest_value: &'a IOValue,
    pub member_refs: &'a [(String, String)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub promotion_summary_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNodeDogfoodInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNodeDogfoodRun {
    pub decision: String,
    pub workflow_ref: String,
    pub workflow_value: IOValue,
    pub step_values: Vec<IOValue>,
    pub checkpoint_values: Vec<IOValue>,
    pub report_ref: String,
    pub report_value: IOValue,
    pub release_gate_ref: Option<String>,
    pub release_gate_value: Option<IOValue>,
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
    pub value: IOValue,
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
    pub value: IOValue,
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
    pub value: IOValue,
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
    pub value: IOValue,
}

pub fn operator_step_value(input: &OperatorStepInput<'_>) -> Result<IOValue> {
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
    Ok(record("operator-step-v1", vec![
        string(OPERATOR_STEP_SCHEMA),
        record("name", vec![string(input.name)]),
        record("request", vec![optional_ref_value(input.request_ref)]),
        record("receipt", vec![optional_ref_value(input.receipt_ref)]),
        record("decision", vec![string(input.decision)]),
        record("replay", vec![string(input.replay_status)]),
        record("mandatory", vec![bool_value(input.mandatory)]),
        record("artifacts", vec![refs_sequence(input.artifact_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-step", "pass"),
            ("explicit-request-ref", status(input.request_ref.is_some())),
            ("canonical-receipt-ref", status(has_receipt)),
            ("mandatory-classification", mandatory_status),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_operator_step(value: &IOValue) -> Result<OperatorStep> {
    let fields = value
        .collect_simple_record("operator-step-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-step-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_STEP_SCHEMA, "operator step")?;
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
        step_ref: canonical_hash(value)?,
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

pub fn operator_checkpoint_value(input: &OperatorCheckpointInput<'_>) -> Result<IOValue> {
    validate_workflow_id(input.workflow_id)?;
    validate_ref(input.step_ref, "operator checkpoint step ref")?;
    validate_optional_ref(input.request_ref, "operator checkpoint request ref")?;
    validate_optional_ref(input.receipt_ref, "operator checkpoint receipt ref")?;
    validate_optional_ref(input.result_ref, "operator checkpoint result ref")?;
    validate_ref(input.state_root_ref, "operator checkpoint state root ref")?;
    Ok(record("operator-checkpoint-v1", vec![
        string(OPERATOR_CHECKPOINT_SCHEMA),
        record("workflow", vec![string(input.workflow_id)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("step", vec![string(input.step_ref)]),
        record("request", vec![optional_ref_value(input.request_ref)]),
        record("receipt", vec![optional_ref_value(input.receipt_ref)]),
        record("result", vec![optional_ref_value(input.result_ref)]),
        record("state-root", vec![string(input.state_root_ref)]),
        checks_value_from_pairs(&[
            ("checkpoint-after-step", "pass"),
            ("request-receipt-result-bound", status(input.receipt_ref.is_some() && input.result_ref.is_some())),
            ("explicit-state-root", "pass"),
        ]),
    ]))
}

pub fn parse_operator_checkpoint(value: &IOValue) -> Result<OperatorCheckpoint> {
    let fields = value
        .collect_simple_record("operator-checkpoint-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-checkpoint-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_CHECKPOINT_SCHEMA, "operator checkpoint")?;
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
        checkpoint_ref: canonical_hash(value)?,
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

pub fn operator_workflow_value(input: &OperatorWorkflowInput<'_>) -> Result<IOValue> {
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
    Ok(record("operator-workflow-v1", vec![
        string(OPERATOR_WORKFLOW_SCHEMA),
        record("workflow-id", vec![string(input.workflow_id)]),
        record("steps", vec![sequence(input.steps.to_vec())]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("capability", vec![refs_sequence(input.capability_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("replay-profile", vec![string(input.replay_profile)]),
        checks_value_from_pairs(&[
            ("canonical-workflow", "pass"),
            ("no-hidden-bypass", status(!has_hidden_bypass)),
            ("explicit-operator-authority", status(has_mandatory_step_authority)),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_operator_workflow(value: &IOValue) -> Result<OperatorWorkflow> {
    let fields = value
        .collect_simple_record("operator-workflow-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <operator-workflow-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_WORKFLOW_SCHEMA, "operator workflow")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-workflow", "operator workflow")?;
    require_check(&checks, "no-hidden-bypass", "operator workflow")?;
    require_check(&checks, "no-text-oracle", "operator workflow")?;
    let step_values = record_iovalue_sequence(&fields[2], "steps")?;
    ensure_count_at_most(step_values.len(), MAX_OPERATOR_STEPS, "operator workflow steps")?;
    let steps = step_values.iter().map(parse_operator_step).collect::<Result<Vec<_>>>()?;
    Ok(OperatorWorkflow {
        workflow_ref: canonical_hash(value)?,
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

pub fn dogfood_report_value(input: &DogfoodReportInput<'_>) -> Result<IOValue> {
    let workflow = parse_operator_workflow(input.workflow_value)?;
    let checkpoint_refs = input.checkpoint_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    validate_refs(input.gate_receipt_refs, "dogfood gate receipt ref")?;
    validate_refs(input.repro_bundle_refs, "dogfood repro bundle ref")?;
    validate_ref(input.final_state_ref, "dogfood final state ref")?;
    ensure_count_at_most(checkpoint_refs.len(), MAX_OPERATOR_STEPS, "dogfood checkpoints")?;
    let mut diagnostics = input.diagnostics.to_vec();
    ensure_count_at_most(diagnostics.len(), MAX_OPERATOR_DIAGNOSTICS, "dogfood report diagnostics")?;
    let mut step_receipts = Vec::new();
    for step in &workflow.steps {
        if let Some(receipt_ref) = step.receipt_ref.as_ref() {
            step_receipts.push_limited_value(
                (step.name.clone(), receipt_ref.clone()),
                MAX_OPERATOR_STEPS,
                "dogfood step receipts",
            )?;
        }
        for diagnostic in &step.diagnostics {
            diagnostics.push_limited_value(
                format!("dogfood step {} diagnostic: {diagnostic}", step.name),
                MAX_OPERATOR_DIAGNOSTICS,
                "dogfood report diagnostics",
            )?;
        }
        if step.mandatory && step.receipt_ref.is_none() {
            diagnostics.push_limited_value(
                format!("mandatory dogfood step {} lacks canonical receipt", step.name),
                MAX_OPERATOR_DIAGNOSTICS,
                "dogfood report diagnostics",
            )?;
        }
        if step.mandatory && step.decision != "pass" {
            diagnostics.push_limited_value(
                format!("mandatory dogfood step {} decision is {}", step.name, step.decision),
                MAX_OPERATOR_DIAGNOSTICS,
                "dogfood report diagnostics",
            )?;
        }
        if step.mandatory && !matches!(step.replay_status.as_str(), "deterministic" | "recorded") {
            diagnostics.push_limited_value(
                format!("mandatory dogfood step {} has non-release replay status {}", step.name, step.replay_status),
                MAX_OPERATOR_DIAGNOSTICS,
                "dogfood report diagnostics",
            )?;
        }
    }
    if checkpoint_refs.len() < workflow.steps.len() {
        diagnostics.push_limited_value(
            format!(
                "dogfood workflow has {} steps but only {} checkpoints",
                workflow.steps.len(),
                checkpoint_refs.len()
            ),
            MAX_OPERATOR_DIAGNOSTICS,
            "dogfood report diagnostics",
        )?;
    }
    if !workflow_check_pass(&workflow.checks, "no-hidden-bypass") {
        diagnostics.push_limited_value(
            "dogfood workflow contains hidden or unreceipted operator bypass".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "dogfood report diagnostics",
        )?;
    }
    if !workflow_check_pass(&workflow.checks, "explicit-operator-authority") {
        diagnostics.push_limited_value(
            "dogfood workflow lacks current explicit operator policy/capability refs".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "dogfood report diagnostics",
        )?;
    }
    if input.gate_receipt_refs.is_empty() {
        diagnostics.push_limited_value(
            "dogfood report requires at least one gate receipt".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "dogfood report diagnostics",
        )?;
    }
    if input.repro_bundle_refs.is_empty() {
        diagnostics.push_limited_value(
            "dogfood report requires a sealed/redacted repro bundle ref".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "dogfood report diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("dogfood-report-v1", vec![
        string(OPERATOR_DOGFOOD_REPORT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("workflow", vec![string(&workflow.workflow_ref)]),
        record("checkpoints", vec![refs_sequence(&checkpoint_refs)]),
        record("step-receipts", vec![step_receipts_sequence(&step_receipts)]),
        record("gate-receipts", vec![refs_sequence(input.gate_receipt_refs)]),
        record("repro-bundles", vec![refs_sequence(input.repro_bundle_refs)]),
        record("final-state", vec![string(input.final_state_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-report", "pass"),
            ("deterministic-or-recorded", status(diagnostics.iter().all(|item| !item.contains("replay status")))),
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

pub fn parse_dogfood_report(value: &IOValue) -> Result<DogfoodReport> {
    let fields = value
        .collect_simple_record("dogfood-report-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dogfood-report-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_DOGFOOD_REPORT_SCHEMA, "dogfood report")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-report", "dogfood report")?;
    require_check(&checks, "final-state-bound", "dogfood report")?;
    require_check(&checks, "no-text-oracle", "dogfood report")?;
    Ok(DogfoodReport {
        report_ref: canonical_hash(value)?,
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

pub fn release_gate_receipt_value(input: &ReleaseGateInput<'_>) -> Result<IOValue> {
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
    require_non_empty_refs(input.retention_gc_refs, "dogfood release retention GC ref")?;
    require_non_empty_refs(input.validation_command_refs, "dogfood release validation command ref")?;
    Ok(record("release-gate-receipt-v1", vec![
        string(OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("report", vec![string(&report.report_ref)]),
        record("node", vec![string(input.node_startup_ref), string(input.node_shutdown_ref)]),
        record("harness-gates", vec![refs_sequence(input.harness_gate_refs)]),
        record("catalog-queries", vec![refs_sequence(input.catalog_query_refs)]),
        record("repro-verifies", vec![refs_sequence(input.repro_verify_refs)]),
        record("retention-gc", vec![refs_sequence(input.retention_gc_refs)]),
        record("validation-commands", vec![refs_sequence(input.validation_command_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("deterministic-or-recorded-only", "pass"),
            ("redaction-gate-bound", "pass"),
            ("startup-shutdown-bound", "pass"),
            ("catalog-mcp-bound", "pass"),
            ("retention-gc-review-bound", "pass"),
            ("retention-gc-is-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_gate_receipt(value: &IOValue) -> Result<ReleaseGateReceipt> {
    let fields = value
        .collect_simple_record("release-gate-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA, "operator release gate")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "dogfood-report-pass", "operator release gate")?;
    require_check(&checks, "no-text-oracle", "operator release gate")?;
    let node = value_to_iovalue(&fields[3]);
    let node_fields = simple_record(&node, "node", 2)?;
    Ok(ReleaseGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        report_ref: record_ref(&fields[2], "report")?,
        startup_ref: required_ref(&node_fields[0], "release gate startup ref")?,
        shutdown_ref: required_ref(&node_fields[1], "release gate shutdown ref")?,
        harness_gate_refs: record_ref_sequence(&fields[4], "harness-gates")?,
        catalog_query_refs: record_ref_sequence(&fields[5], "catalog-queries")?,
        repro_verify_refs: record_ref_sequence(&fields[6], "repro-verifies")?,
        retention_gc_refs: record_ref_sequence(&fields[7], "retention-gc")?,
        validation_command_refs: record_ref_sequence(&fields[8], "validation-commands")?,
        checks,
        value: value.clone(),
    })
}

pub fn nix_dogfood_release_evidence_value(input: &NixDogfoodEvidenceInput<'_>) -> Result<IOValue> {
    let observed = observe_nix_dogfood_output(input.output_path)?;
    Ok(record("nix-dogfood-release-evidence-v1", vec![
        string(OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA),
        record("output-path", vec![string(observed.output_path.as_str()), string(&observed.output_path_ref)]),
        record("report", vec![string(&observed.report_ref)]),
        record("release-gate", vec![string(&observed.release_gate_ref)]),
        record("summary", vec![string(&observed.summary_ref)]),
        record("nextest", vec![
            string(&observed.nextest_marker_ref),
            string(observed.nextest_check_path.as_str()),
        ]),
        record("files", vec![file_refs_sequence(&observed.file_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-ref-bound", "pass"),
            ("nix-output-path-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_nix_dogfood_evidence(value: &IOValue) -> Result<NixDogfoodEvidence> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-evidence-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-evidence-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_NIX_DOGFOOD_EVIDENCE_SCHEMA, "Nix dogfood evidence")?;
    let output_path = value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let nextest = value_to_iovalue(&fields[5]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "release-evidence-only", "Nix dogfood evidence")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood evidence")?;
    Ok(NixDogfoodEvidence {
        evidence_ref: canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "Nix dogfood output path")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood output path ref")?,
        report_ref: record_ref(&fields[2], "report")?,
        release_gate_ref: record_ref(&fields[3], "release-gate")?,
        summary_ref: record_ref(&fields[4], "summary")?,
        nextest_marker_ref: required_ref(&nextest_fields[0], "Nix dogfood nextest marker ref")?,
        nextest_check_path: required_string(&nextest_fields[1], "Nix dogfood nextest check path")?,
        file_refs: record_file_refs(&fields[6], "files")?,
        checks,
        value: value.clone(),
    })
}

pub fn verify_nix_dogfood_evidence(input: &NixDogfoodVerifyInput<'_>) -> Result<NixDogfoodVerifyReceipt> {
    let evidence = parse_nix_dogfood_evidence(input.evidence_value)?;
    let observed_result = observe_nix_dogfood_output(input.output_path);
    let mut diagnostics = Vec::new();
    let output_path_string = input.output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let mut is_output_observed = true;
    let observed = match observed_result {
        Ok(observed) => observed,
        Err(error) => {
            is_output_observed = false;
            diagnostics.push_limited_value(
                format!("Nix dogfood output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "Nix dogfood verify diagnostics",
            )?;
            ObservedNixDogfoodOutput {
                output_path: output_path_string,
                output_path_ref: fallback_output_path_ref,
                report_ref: evidence.report_ref.clone(),
                release_gate_ref: evidence.release_gate_ref.clone(),
                summary_ref: evidence.summary_ref.clone(),
                nextest_marker_ref: evidence.nextest_marker_ref.clone(),
                nextest_check_path: evidence.nextest_check_path.clone(),
                file_refs: evidence.file_refs.clone(),
            }
        }
    };
    for diagnostic in [
        mismatch_diagnostic("output-path-ref", &evidence.output_path_ref, &observed.output_path_ref),
        mismatch_diagnostic("report-ref", &evidence.report_ref, &observed.report_ref),
        mismatch_diagnostic("release-gate-ref", &evidence.release_gate_ref, &observed.release_gate_ref),
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
    let value = record("nix-dogfood-release-verify-receipt-v1", vec![
        string(OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("evidence", vec![string(&evidence.evidence_ref)]),
        record("output-path", vec![string(observed.output_path.as_str()), string(&observed.output_path_ref)]),
        record("report", vec![string(&observed.report_ref)]),
        record("release-gate", vec![string(&observed.release_gate_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-ref-bound", status(evidence.release_gate_ref == observed.release_gate_ref)),
            ("nix-output-path-bound", status(evidence.output_path_ref == observed.output_path_ref)),
            ("nextest-dependency-bound", status(evidence.nextest_marker_ref == observed.nextest_marker_ref)),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]);
    parse_nix_dogfood_verify_receipt(&value)
}

pub fn parse_nix_dogfood_verify_receipt(value: &IOValue) -> Result<NixDogfoodVerifyReceipt> {
    let fields = value
        .collect_simple_record("nix-dogfood-release-verify-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <nix-dogfood-release-verify-receipt-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_NIX_DOGFOOD_VERIFY_RECEIPT_SCHEMA, "Nix dogfood verify receipt")?;
    let output_path = value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "release-evidence-only", "Nix dogfood verify receipt")?;
    require_check(&checks, "no-text-oracle", "Nix dogfood verify receipt")?;
    Ok(NixDogfoodVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        evidence_ref: record_ref(&fields[2], "evidence")?,
        output_path_ref: required_ref(&output_fields[1], "Nix dogfood verify output path ref")?,
        report_ref: record_ref(&fields[4], "report")?,
        release_gate_ref: record_ref(&fields[5], "release-gate")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_evidence_bundle_value(input: &ReleaseEvidenceBundleInput<'_>) -> Result<IOValue> {
    let observed = observe_release_bundle_output(input.output_path)?;
    Ok(record("release-evidence-bundle-v1", vec![
        string(OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA),
        record("output-path", vec![string(observed.output_path.as_str()), string(&observed.output_path_ref)]),
        record("members", vec![file_refs_sequence(&observed.member_refs)]),
        record("dogfood", vec![string(&observed.report_ref), string(&observed.release_gate_ref)]),
        record("nix", vec![string(&observed.nix_evidence_ref), string(&observed.nix_verify_ref)]),
        record("nextest", vec![
            string(&observed.nextest_marker_ref),
            string(observed.nextest_check_path.as_str()),
        ]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("release-gate-pass", "pass"),
            ("nix-verify-pass", "pass"),
            ("bundle-members-bound", "pass"),
            ("nextest-dependency-bound", "pass"),
            ("release-evidence-only", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
}

pub fn parse_release_evidence_bundle(value: &IOValue) -> Result<ReleaseEvidenceBundle> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA, "release evidence bundle")?;
    let output_path = value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = value_to_iovalue(&fields[3]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let nix = value_to_iovalue(&fields[4]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let nextest = value_to_iovalue(&fields[5]);
    let nextest_fields = simple_record(&nextest, "nextest", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle")?;
    Ok(ReleaseEvidenceBundle {
        bundle_ref: canonical_hash(value)?,
        output_path: required_string(&output_fields[0], "release evidence output path")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence release gate ref")?,
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

pub fn verify_release_evidence_bundle(
    input: &ReleaseEvidenceBundleVerifyInput<'_>,
) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let bundle = parse_release_evidence_bundle(input.bundle_value)?;
    let observed_result = observe_release_bundle_output(input.output_path);
    let mut diagnostics = Vec::new();
    let output_path_string = input.output_path.display().to_string();
    let fallback_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let mut is_output_observed = true;
    let observed = match observed_result {
        Ok(observed) => observed,
        Err(error) => {
            is_output_observed = false;
            diagnostics.push_limited_value(
                format!("release evidence bundle output observation failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release evidence bundle verify diagnostics",
            )?;
            ObservedReleaseBundleOutput {
                output_path: output_path_string,
                output_path_ref: fallback_output_path_ref,
                report_ref: bundle.report_ref.clone(),
                release_gate_ref: bundle.release_gate_ref.clone(),
                nix_evidence_ref: bundle.nix_evidence_ref.clone(),
                nix_verify_ref: bundle.nix_verify_ref.clone(),
                summary_ref: bundle.summary_ref.clone(),
                nextest_marker_ref: bundle.nextest_marker_ref.clone(),
                nextest_check_path: bundle.nextest_check_path.clone(),
                member_refs: bundle.member_refs.clone(),
            }
        }
    };
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
    let value = record("release-evidence-bundle-verify-receipt-v1", vec![
        string(OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("bundle", vec![string(&bundle.bundle_ref)]),
        record("output-path", vec![string(observed.output_path.as_str()), string(&observed.output_path_ref)]),
        record("dogfood", vec![string(&observed.report_ref), string(&observed.release_gate_ref)]),
        record("nix", vec![string(&observed.nix_evidence_ref), string(&observed.nix_verify_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", status(is_output_observed)),
            ("release-gate-pass", status(is_output_observed)),
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

pub fn parse_release_evidence_bundle_verify_receipt(value: &IOValue) -> Result<ReleaseEvidenceBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-evidence-bundle-verify-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-evidence-bundle-verify-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        OPERATOR_RELEASE_EVIDENCE_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "release evidence bundle verify receipt",
    )?;
    let output_path = value_to_iovalue(&fields[3]);
    let output_fields = simple_record(&output_path, "output-path", 2)?;
    let dogfood = value_to_iovalue(&fields[4]);
    let dogfood_fields = simple_record(&dogfood, "dogfood", 2)?;
    let nix = value_to_iovalue(&fields[5]);
    let nix_fields = simple_record(&nix, "nix", 2)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "bundle-members-bound", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-member-receipts", "release evidence bundle verify receipt")?;
    require_check(&checks, "signed-receipts-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "release-evidence-only", "release evidence bundle verify receipt")?;
    require_check(&checks, "no-text-oracle", "release evidence bundle verify receipt")?;
    Ok(ReleaseEvidenceBundleVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        bundle_ref: record_ref(&fields[2], "bundle")?,
        output_path_ref: required_ref(&output_fields[1], "release evidence verify output path ref")?,
        report_ref: required_ref(&dogfood_fields[0], "release evidence verify report ref")?,
        release_gate_ref: required_ref(&dogfood_fields[1], "release evidence verify release gate ref")?,
        nix_evidence_ref: required_ref(&nix_fields[0], "release evidence verify Nix evidence ref")?,
        nix_verify_ref: required_ref(&nix_fields[1], "release evidence verify Nix verify ref")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn release_promotion_gate_receipt_value(
    input: &ReleasePromotionGateInput<'_>,
) -> Result<ReleasePromotionGateReceipt> {
    let bundle_verify = parse_release_evidence_bundle_verify_receipt(input.bundle_verify_value)?;
    let output_path_string = input.output_path.display().to_string();
    let observed_output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let source_ref = raw_text_ref("molten.operator.release-promotion.source-evidence.v1", input.source_evidence);
    let octet_ref = raw_text_ref("molten.operator.release-promotion.octet-evidence.v1", input.octet_evidence);
    let cairn_ref = raw_text_ref("molten.operator.release-promotion.cairn-evidence.v1", input.cairn_evidence);
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
    if observed_output_path_ref != bundle_verify.output_path_ref {
        diagnostics.push_limited_value(
            format!(
                "promotion output-path-ref mismatch: receipt={} observed={}",
                bundle_verify.output_path_ref, observed_output_path_ref
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
    let selected_key = match select_release_promotion_key(input) {
        Ok(key) => Some(key),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("signed keyring currentness failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion diagnostics",
            )?;
            None
        }
    };
    let selected_key_ref =
        selected_key.map_or_else(|| "blake3:missing-signed-key".to_string(), |key| key.key_ref.clone());
    let selected_key_id = selected_key.map_or_else(|| "missing".to_string(), |key| key.key_id.clone());
    let selected_signer =
        selected_key.map_or_else(|| input.signed_signer.unwrap_or("missing").to_string(), |key| key.signer.clone());
    let selected_trust_root =
        selected_key.map_or_else(|| input.signed_trust_root.to_string(), |key| key.trust_root.clone());
    let selected_generation = selected_key.map_or(0, |key| key.generation);
    let key_revocation_refs = input
        .signed_key_revocations
        .iter()
        .map(|revocation| revocation.revocation_ref.clone())
        .collect::<Vec<_>>();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("release-promotion-gate-receipt-v1", vec![
        string(OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("bundle-verify", vec![
            string(&bundle_verify.receipt_ref),
            string(&bundle_verify.bundle_ref),
            string(&bundle_verify.output_path_ref),
            string(&bundle_verify.report_ref),
            string(&bundle_verify.release_gate_ref),
            string(&bundle_verify.nix_evidence_ref),
            string(&bundle_verify.nix_verify_ref),
        ]),
        record("signed-keyring", vec![
            record("selected-key", vec![
                string(&selected_key_ref),
                string(&selected_key_id),
                string(&selected_signer),
                string(&selected_trust_root),
                u64_value(selected_generation),
            ]),
            refs_sequence(&key_revocation_refs),
        ]),
        record("evidence", vec![
            record("source", vec![string(input.source_evidence), string(&source_ref)]),
            record("octet", vec![string(input.octet_evidence), string(&octet_ref)]),
            record("cairn", vec![string(input.cairn_evidence), string(&cairn_ref)]),
        ]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("release-bundle-verify-pass", status(bundle_verify.decision == "pass")),
            ("promotion-output-path-bound", status(observed_output_path_ref == bundle_verify.output_path_ref)),
            ("signed-keyring-current", status(selected_key.is_some())),
            ("source-evidence-bound", status(!input.source_evidence.trim().is_empty())),
            ("octet-evidence-bound", status(!input.octet_evidence.trim().is_empty())),
            ("cairn-evidence-bound", status(!input.cairn_evidence.trim().is_empty())),
            ("release-promotion-is-evidence-only", "pass"),
            ("no-subsystem-authority-granted", "pass"),
        ]),
    ]);
    parse_release_promotion_gate_receipt(&value)
}

pub fn parse_release_promotion_gate_receipt(value: &IOValue) -> Result<ReleasePromotionGateReceipt> {
    let fields = value
        .collect_simple_record("release-promotion-gate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_PROMOTION_GATE_RECEIPT_SCHEMA, "release promotion gate receipt")?;
    let bundle_value = value_to_iovalue(&fields[2]);
    let bundle_fields = simple_record(&bundle_value, "bundle-verify", 7)?;
    let keyring_value = value_to_iovalue(&fields[3]);
    let keyring_fields = simple_record(&keyring_value, "signed-keyring", 2)?;
    let selected_key_value = value_to_iovalue(&keyring_fields[0]);
    let selected_key_fields = simple_record(&selected_key_value, "selected-key", 5)?;
    let evidence_value = value_to_iovalue(&fields[4]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 2)?;
    let octet_value = value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 2)?;
    let cairn_value = value_to_iovalue(&evidence_fields[2]);
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
        receipt_ref: canonical_hash(value)?,
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

pub fn release_promotion_summary_value(input: &ReleasePromotionSummaryInput<'_>) -> Result<ReleasePromotionSummary> {
    let output_path_string = input.output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let mut diagnostics = Vec::new();

    let promotion_result = read_output_text(input.output_path, "release-promotion-gate.preserves")
        .and_then(|text| parse_text(&text))
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
    }

    let expected_subject_ref = promotion.as_ref().map(|promotion| promotion.receipt_ref.as_str());
    let signed_result = read_output_text(input.output_path, "release-promotion-gate.signed.preserves")
        .and_then(|text| parse_text(&text))
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
    let signed = match signed_result {
        Ok(signed) => Some(signed),
        Err(error) => {
            diagnostics.push_limited_value(
                format!("signed promotion receipt verification failed: {error}"),
                MAX_OPERATOR_DIAGNOSTICS,
                "release promotion summary diagnostics",
            )?;
            None
        }
    };

    let promotion_ref = promotion
        .as_ref()
        .map_or_else(|| "blake3:missing-release-promotion-gate".to_string(), |promotion| promotion.receipt_ref.clone());
    let promotion_decision = promotion.as_ref().map_or("missing", |promotion| promotion.decision.as_str());
    let bundle_verify_ref = promotion.as_ref().map_or_else(
        || "blake3:missing-release-bundle-verify".to_string(),
        |promotion| promotion.bundle_verify_ref.clone(),
    );
    let bundle_ref = promotion
        .as_ref()
        .map_or_else(|| "blake3:missing-release-evidence-bundle".to_string(), |promotion| promotion.bundle_ref.clone());
    let source_ref = promotion
        .as_ref()
        .map_or_else(|| "blake3:missing-source-evidence".to_string(), |promotion| promotion.source_ref.clone());
    let octet_ref = promotion
        .as_ref()
        .map_or_else(|| "blake3:missing-octet-evidence".to_string(), |promotion| promotion.octet_ref.clone());
    let cairn_ref = promotion
        .as_ref()
        .map_or_else(|| "blake3:missing-cairn-evidence".to_string(), |promotion| promotion.cairn_ref.clone());
    let signed_envelope_ref = signed.as_ref().map_or_else(
        || "blake3:missing-signed-release-promotion".to_string(),
        |signed| signed.receipt.envelope_ref.clone(),
    );
    let signed_subject_ref = signed.as_ref().map_or_else(
        || "blake3:missing-signed-release-promotion-subject".to_string(),
        |signed| signed.receipt.subject_ref.clone(),
    );
    let signed_key_ref = signed
        .as_ref()
        .map_or_else(|| "blake3:missing-signed-release-key".to_string(), |signed| signed.key_ref.clone());
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("release-promotion-summary-v1", vec![
        string(OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA),
        record("decision", vec![string(decision)]),
        record("output", vec![string(&output_path_string), string(&output_path_ref)]),
        record("promotion", vec![
            string(&promotion_ref),
            string(promotion_decision),
            string(&bundle_verify_ref),
            string(&bundle_ref),
        ]),
        record("signed-promotion", vec![
            string(&signed_envelope_ref),
            string(&signed_subject_ref),
            string(&signed_key_ref),
            string(RELEASE_PROMOTION_SIGNING_PURPOSE),
        ]),
        record("evidence", vec![
            record("source", vec![string(&source_ref)]),
            record("octet", vec![string(&octet_ref)]),
            record("cairn", vec![string(&cairn_ref)]),
        ]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            (
                "release-promotion-pass",
                status(promotion.as_ref().is_some_and(|promotion| promotion.decision == "pass")),
            ),
            (
                "release-promotion-output-bound",
                status(promotion.as_ref().is_some_and(|promotion| promotion.output_path_ref == output_path_ref)),
            ),
            ("signed-promotion-present", status(signed.is_some())),
            (
                "signed-promotion-subject-bound",
                status(signed.as_ref().is_some_and(|signed| signed.receipt.subject_ref == promotion_ref)),
            ),
            ("signed-promotion-keyring-current", status(signed.is_some())),
            ("release-promotion-summary-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_promotion_summary(&value)
}

pub fn parse_release_promotion_summary(value: &IOValue) -> Result<ReleasePromotionSummary> {
    let fields = value
        .collect_simple_record("release-promotion-summary-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-promotion-summary-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_PROMOTION_SUMMARY_SCHEMA, "release promotion summary")?;
    let promotion_value = value_to_iovalue(&fields[3]);
    let promotion_fields = simple_record(&promotion_value, "promotion", 4)?;
    let signed_value = value_to_iovalue(&fields[4]);
    let signed_fields = simple_record(&signed_value, "signed-promotion", 4)?;
    let evidence_value = value_to_iovalue(&fields[5]);
    let evidence_fields = simple_record(&evidence_value, "evidence", 3)?;
    let source_value = value_to_iovalue(&evidence_fields[0]);
    let source_fields = simple_record(&source_value, "source", 1)?;
    let octet_value = value_to_iovalue(&evidence_fields[1]);
    let octet_fields = simple_record(&octet_value, "octet", 1)?;
    let cairn_value = value_to_iovalue(&evidence_fields[2]);
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
        summary_ref: canonical_hash(value)?,
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
    let summary_value = parse_text(&read_output_text(input.output_path, "release-promotion-summary.preserves")?)?;
    let summary = parse_release_promotion_summary(&summary_value)?;
    if summary.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "release export requires pass promotion summary {}; decision is {}",
            summary.summary_ref, summary.decision
        )));
    }
    let member_refs = observe_release_export_members(input.output_path)?;
    let value = record("release-export-manifest-v1", vec![
        string(OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA),
        record("output", vec![string(&output_path_string), string(&output_path_ref)]),
        record("promotion-summary", vec![string(&summary.summary_ref)]),
        record("members", vec![file_refs_sequence(&member_refs)]),
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

pub fn parse_release_export_manifest(value: &IOValue) -> Result<ReleaseExportManifest> {
    let fields = value
        .collect_simple_record("release-export-manifest-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-manifest-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_EXPORT_MANIFEST_SCHEMA, "release export manifest")?;
    let output_value = value_to_iovalue(&fields[1]);
    let output_fields = simple_record(&output_value, "output", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-promotion-summary-pass", "release export manifest")?;
    require_check(&checks, "release-export-members-bound", "release export manifest")?;
    require_check(&checks, "deterministic-archive-layout", "release export manifest")?;
    require_check(&checks, "release-export-is-evidence-only", "release export manifest")?;
    require_check(&checks, "no-release-authority-granted", "release export manifest")?;
    Ok(ReleaseExportManifest {
        manifest_ref: canonical_hash(value)?,
        output_path_ref: required_ref(&output_fields[1], "release export output path ref")?,
        promotion_summary_ref: record_ref(&fields[2], "promotion-summary")?,
        member_refs: record_file_refs(&fields[3], "members")?,
        checks,
        value: value.clone(),
    })
}

pub fn verify_release_export(input: &ReleaseExportVerifyInput<'_>) -> Result<ReleaseExportVerifyReceipt> {
    let manifest = parse_release_export_manifest(input.manifest_value)?;
    let mut diagnostics = file_ref_mismatch_diagnostics(&manifest.member_refs, input.member_refs)?;
    if input.member_refs.iter().any(|(name, _)| name == "release-export-manifest.preserves") {
        diagnostics.push_limited_value(
            "release export archive must not list manifest as a payload member".to_string(),
            MAX_OPERATOR_DIAGNOSTICS,
            "release export verify diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("release-export-verify-receipt-v1", vec![
        string(OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![string(&manifest.manifest_ref), string(&manifest.promotion_summary_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("release-export-members-bound", status(diagnostics.is_empty())),
            ("release-promotion-summary-bound", status(diagnostics.is_empty())),
            ("release-export-is-evidence-only", "pass"),
            ("no-release-authority-granted", "pass"),
        ]),
    ]);
    parse_release_export_verify_receipt(&value)
}

pub fn parse_release_export_verify_receipt(value: &IOValue) -> Result<ReleaseExportVerifyReceipt> {
    let fields = value
        .collect_simple_record("release-export-verify-receipt-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <release-export-verify-receipt-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_RELEASE_EXPORT_VERIFY_RECEIPT_SCHEMA, "release export verify receipt")?;
    let manifest_value = value_to_iovalue(&fields[2]);
    let manifest_fields = simple_record(&manifest_value, "manifest", 2)?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "release-export-members-bound", "release export verify receipt")?;
    require_check(&checks, "release-promotion-summary-bound", "release export verify receipt")?;
    require_check(&checks, "release-export-is-evidence-only", "release export verify receipt")?;
    require_check(&checks, "no-release-authority-granted", "release export verify receipt")?;
    Ok(ReleaseExportVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
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
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    file_refs: Vec<(String, String)>,
}

fn observe_nix_dogfood_output(output_path: &Path) -> Result<ObservedNixDogfoodOutput> {
    let output_path_string = output_path.display().to_string();
    let output_path_ref = raw_text_ref("molten.operator.nix-dogfood-output-path.v1", &output_path_string);
    let report_text = read_output_text(output_path, "dogfood-report.preserves")?;
    let release_gate_text = read_output_text(output_path, "release-gate.preserves")?;
    let summary_text = read_output_text(output_path, "dogfood-summary.txt")?;
    let nextest_text = read_output_text(output_path, "after-nextest.txt")?;
    let report_value = parse_text(&report_text)?;
    let release_gate_value = parse_text(&release_gate_text)?;
    let report = parse_dogfood_report(&report_value)?;
    let release_gate = parse_release_gate_receipt(&release_gate_value)?;
    if report.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass report {}; decision is {}",
            report.report_ref, report.decision
        )));
    }
    if release_gate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood evidence requires pass release gate {}; decision is {}",
            release_gate.receipt_ref, release_gate.decision
        )));
    }
    if release_gate.report_ref != report.report_ref {
        return Err(MoltenError::invalid_harness(format!(
            "Nix dogfood release gate report ref {} does not match report {}",
            release_gate.report_ref, report.report_ref
        )));
    }
    let nextest_check_path = nextest_text.trim().to_string();
    if nextest_check_path.is_empty() {
        return Err(MoltenError::invalid_harness("Nix dogfood after-nextest marker is empty"));
    }
    let summary_ref = raw_text_ref("molten.operator.nix-dogfood-summary.v1", &summary_text);
    let nextest_marker_ref = raw_text_ref("molten.operator.nix-dogfood-nextest-marker.v1", &nextest_text);
    let mut file_refs = Vec::new();
    file_refs.push_limited_value(
        ("dogfood-report.preserves".to_string(), report.report_ref.clone()),
        MAX_OPERATOR_REFS,
        "Nix dogfood file refs",
    )?;
    file_refs.push_limited_value(
        ("release-gate.preserves".to_string(), release_gate.receipt_ref.clone()),
        MAX_OPERATOR_REFS,
        "Nix dogfood file refs",
    )?;
    file_refs.push_limited_value(
        ("dogfood-summary.txt".to_string(), summary_ref.clone()),
        MAX_OPERATOR_REFS,
        "Nix dogfood file refs",
    )?;
    file_refs.push_limited_value(
        ("after-nextest.txt".to_string(), nextest_marker_ref.clone()),
        MAX_OPERATOR_REFS,
        "Nix dogfood file refs",
    )?;
    Ok(ObservedNixDogfoodOutput {
        output_path: output_path_string,
        output_path_ref,
        report_ref: report.report_ref,
        release_gate_ref: release_gate.receipt_ref,
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
    nix_evidence_ref: String,
    nix_verify_ref: String,
    summary_ref: String,
    nextest_marker_ref: String,
    nextest_check_path: String,
    member_refs: Vec<(String, String)>,
}

fn observe_release_bundle_output(output_path: &Path) -> Result<ObservedReleaseBundleOutput> {
    let observed_nix = observe_nix_dogfood_output(output_path)?;
    let nix_evidence_value = parse_text(&read_output_text(output_path, "nix-dogfood-evidence.preserves")?)?;
    let nix_verify_value = parse_text(&read_output_text(output_path, "nix-dogfood-verify.preserves")?)?;
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
    signed_value: &IOValue,
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
        let bytes = fs::read(output_path.join(name)).map_err(MoltenError::from)?;
        members.push_limited_value(
            (name.to_string(), release_export_file_ref(name, &bytes)),
            MAX_OPERATOR_REFS,
            "release export members",
        )?;
    }
    for name in release_export_keyring_member_names(output_path)? {
        let bytes = fs::read(output_path.join(&name)).map_err(MoltenError::from)?;
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
        for entry in fs::read_dir(path).map_err(MoltenError::from)? {
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
    fs::read_to_string(output_path.join(name)).map_err(MoltenError::from)
}

fn raw_text_ref(domain: &str, text: &str) -> String {
    let mut bytes = Vec::with_capacity(domain.len().saturating_add(text.len()).saturating_add(1));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(text.as_bytes());
    format!("blake3:{}", blake3::hash(&bytes).to_hex())
}

fn raw_bytes_ref(domain: &str, name: &str, payload: &[u8]) -> String {
    let mut bytes =
        Vec::with_capacity(domain.len().saturating_add(name.len()).saturating_add(payload.len()).saturating_add(2));
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload);
    format!("blake3:{}", blake3::hash(&bytes).to_hex())
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
    Ok(diagnostics)
}

pub fn run_local_node_dogfood(input: &LocalNodeDogfoodInput<'_>) -> Result<LocalNodeDogfoodRun> {
    let state_root_ref = dogfood_ref("state-root")?;
    if let Some(dirty_reason) = dirty_state_reason(input.state_root)? {
        return dirty_state_report(&state_root_ref, dirty_reason);
    }
    fs::create_dir_all(input.state_root).map_err(MoltenError::from)?;
    let registry_root = input.state_root.join("registry");
    let ledger_root = input.state_root.join("ledger");
    let job_source_root = input.state_root.join("job-source-registry");
    let job_target_root = input.state_root.join("job-target-registry");
    let retention_root = input.state_root.join("retention-store");
    let retention_bundle_root = input.state_root.join("retention-bundle");

    let policy_refs = vec![dogfood_ref("operator-policy")?];
    let capability_refs = vec![dogfood_ref("operator-capability")?];
    let resource_refs = vec![dogfood_ref("operator-resource")?];
    let mut step_checkpoints = StepCheckpointBuffers::default();
    let mut gate_receipt_refs = Vec::new();
    let mut repro_bundle_refs = Vec::new();
    let mut harness_gate_refs = Vec::new();
    let mut catalog_query_refs = Vec::new();
    let mut repro_verify_refs = Vec::new();

    let identity_resolution = resolve_identity(input.state_root, &policy_refs)?;
    let identity = identity_resolution
        .identity
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("local dogfood identity resolution denied"))?;
    let identity_startup =
        node_identity::node_identity_startup_evidence_value(&identity.identity_ref, &identity_resolution.receipt_ref)?;
    let identity_startup_ref = canonical_hash(&identity_startup)?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "clean-state",
        request_ref: Some(&state_root_ref),
        receipt_ref: Some(&identity_resolution.receipt_ref),
        result_ref: Some(&identity_startup_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[identity.identity_ref.clone(), identity_startup_ref.clone()],
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let node_started =
        start_node(&identity, &identity_resolution.receipt_ref, &policy_refs, &capability_refs, &resource_refs)?;
    let startup_ref = node_started.startup_receipt.receipt_ref.clone();
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "start-node",
        request_ref: Some(&node_started.config.config_ref),
        receipt_ref: Some(&startup_ref),
        result_ref: Some(&startup_ref),
        decision: &node_started.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&node_started.config.config_ref),
        diagnostics: &node_started.startup_receipt.diagnostics,
        state_root_ref: &state_root_ref,
    })?;

    let installed = artifacts::install_artifact(&registry_root, &artifacts::ArtifactInstallInput {
        kind: "operator-artifact".to_string(),
        payload: record("dogfood-artifact", vec![string("local-node")]),
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.clone(),
        evidence_refs: vec![startup_ref.clone()],
        installer_ref: capability_refs[0].clone(),
        capability_refs: capability_refs.clone(),
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "install-artifact",
        request_ref: Some(&startup_ref),
        receipt_ref: Some(&canonical_hash(&installed.receipt_value)?),
        result_ref: Some(&installed.artifact_ref),
        decision: &installed.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&installed.artifact_ref),
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let service_suite = crate::service_runtime::two_service_suite_value()?;
    let service_run = crate::service_runtime::run_service_runtime_suite_value(&service_suite)?;
    let service_decision = if service_run.lifecycle_receipts.iter().all(service_lifecycle_pass) {
        "pass"
    } else {
        "deny"
    };
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "start-service",
        request_ref: Some(&canonical_hash(&service_suite)?),
        receipt_ref: Some(&service_run.report_ref),
        result_ref: Some(&service_run.report_ref),
        decision: service_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &service_run.readiness_assertions.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?,
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let remote = remote_dataspace::two_peer_service_ready_harness(
        &input.state_root.join("remote-dataspace"),
        remote_dataspace::RemoteDeliveryEvidence {
            peer_bootstrap_refs: vec![dogfood_ref("remote-peer-bootstrap")?],
            capability_refs: vec![dogfood_ref("remote-capability")?],
            policy_refs: policy_refs.clone(),
            resource_refs: resource_refs.clone(),
            authority_refs: vec![dogfood_ref("remote-authority")?],
        },
    )?;
    let remote_gate_ref = canonical_hash(&remote.gate_receipt_value)?;
    gate_receipt_refs.push_limited_value(remote_gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "publish-remote-assertion",
        request_ref: Some(&remote.delivery_log.log_ref),
        receipt_ref: Some(&remote_gate_ref),
        result_ref: Some(&remote_gate_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&remote.delivery_log.log_ref),
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let job = run_job_stack(JobStackInput {
        state_root: input.state_root,
        source: &job_source_root,
        target: &job_target_root,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        resource_refs: &resource_refs,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "run-job-dag",
        request_ref: Some(&job.execution_request_ref),
        receipt_ref: Some(&job.execution_receipt_ref),
        result_ref: Some(&job.execution_receipt_ref),
        decision: &job.decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &job.artifact_refs,
        diagnostics: &job.diagnostics,
        state_root_ref: &state_root_ref,
    })?;

    let retention_gc = run_retention_gc_workflow(RetentionDogfoodInput {
        root: &retention_root,
        bundle_dir: &retention_bundle_root,
        ledger_root: &ledger_root,
        registry_root: &registry_root,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "plan-retention-gc",
        request_ref: Some(&retention_gc.object_ref),
        receipt_ref: Some(&retention_gc.plan_ref),
        result_ref: Some(&retention_gc.plan_ref),
        decision: &retention_gc.plan_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.plan_ref),
        diagnostics: &retention_gc.plan_diagnostics,
        state_root_ref: &state_root_ref,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "apply-retention-gc-plan",
        request_ref: Some(&retention_gc.plan_ref),
        receipt_ref: Some(&retention_gc.apply_ref),
        result_ref: Some(&retention_gc.apply_ref),
        decision: &retention_gc.apply_decision,
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.apply_ref),
        diagnostics: &retention_gc.apply_diagnostics,
        state_root_ref: &state_root_ref,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "execute-retention-gc",
        request_ref: Some(&retention_gc.apply_ref),
        receipt_ref: Some(&retention_gc.execution_ref),
        result_ref: Some(&retention_gc.execution_ref),
        decision: &retention_gc.execution_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.execution_ref),
        diagnostics: &retention_gc.execution_diagnostics,
        state_root_ref: &state_root_ref,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "audit-retention-gc",
        request_ref: Some(&retention_gc.execution_ref),
        receipt_ref: Some(&retention_gc.audit_ref),
        result_ref: Some(&retention_gc.audit_ref),
        decision: &retention_gc.audit_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&retention_gc.audit_ref),
        diagnostics: &retention_gc.audit_diagnostics,
        state_root_ref: &state_root_ref,
    })?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
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
        state_root_ref: &state_root_ref,
    })?;
    catalog_query_refs.push_limited_value(
        retention_gc.catalog_receipt_ref.clone(),
        MAX_OPERATOR_REFS,
        "catalog query refs",
    )?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "search-retention-gc-catalog",
        request_ref: Some(&retention_gc.catalog_request_ref),
        receipt_ref: Some(&retention_gc.catalog_receipt_ref),
        result_ref: Some(&retention_gc.catalog_response_ref),
        decision: &retention_gc.catalog_decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: &retention_gc.artifact_refs,
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    ledger::import_artifact(&ledger_root, &installed.artifact.value)?;
    ledger::import_artifact(&ledger_root, &remote.gate_receipt_value)?;
    let mcp_request =
        catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string("operator-artifact")])])?;
    let mcp_call = catalog_mcp::call(&registry_root, Some(&ledger_root), &mcp_request)?;
    let mcp_receipt_ref = canonical_hash(&mcp_call.receipt_value)?;
    catalog_query_refs.push_limited_value(mcp_receipt_ref.clone(), MAX_OPERATOR_REFS, "catalog query refs")?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "query-catalog-mcp",
        request_ref: Some(&mcp_call.request.request_ref),
        receipt_ref: Some(&mcp_receipt_ref),
        result_ref: Some(&mcp_call.response_ref),
        decision: &mcp_call.decision,
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&mcp_call.response_ref),
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let repro = build_dogfood_repro()?;
    harness_gate_refs.push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "harness gate refs")?;
    gate_receipt_refs.push_limited_value(repro.gate_ref.clone(), MAX_OPERATOR_REFS, "dogfood gate refs")?;
    repro_bundle_refs.push_limited_value(repro.bundle_ref.clone(), MAX_OPERATOR_REFS, "dogfood repro refs")?;
    repro_verify_refs.push_limited_value(repro.verify_ref.clone(), MAX_OPERATOR_REFS, "dogfood repro verify refs")?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "export-redacted-repro",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.verify_ref),
        result_ref: Some(&repro.bundle_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: &[repro.gate_ref.clone(), repro.bundle_ref.clone()],
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "gate-evidence",
        request_ref: Some(&repro.report_ref),
        receipt_ref: Some(&repro.gate_ref),
        result_ref: Some(&repro.gate_ref),
        decision: "pass",
        replay_status: "deterministic",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&remote_gate_ref),
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let shutdown = node_runtime::node_shutdown_receipt_value(&node_runtime::ShutdownReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup_ref,
        adapter_receipts: &node_started.adapter_receipts,
        drained_job_refs: std::slice::from_ref(&job.execution_receipt_ref),
        index_receipt_refs: &[dogfood_ref("shutdown-index")?],
        diagnostics: &[],
    })?;
    let shutdown_ref = canonical_hash(&shutdown)?;
    let health = node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
        startup_receipt: &node_started.startup_receipt,
        shutdown_receipt_ref: Some(&shutdown_ref),
        index_receipt_refs: &[dogfood_ref("restart-health-index")?],
        head_refs: &[installed.artifact_ref.clone(), job.execution_receipt_ref.clone()],
        open_job_refs: &[],
        diagnostics: &[],
    })?;
    let health_ref = canonical_hash(&health)?;
    push_step_checkpoint(&mut step_checkpoints, StepCheckpointInput {
        name: "shutdown-node",
        request_ref: Some(&startup_ref),
        receipt_ref: Some(&shutdown_ref),
        result_ref: Some(&health_ref),
        decision: "pass",
        replay_status: "recorded",
        mandatory: true,
        artifact_refs: std::slice::from_ref(&health_ref),
        diagnostics: &[],
        state_root_ref: &state_root_ref,
    })?;

    let workflow_value = operator_workflow_value(&OperatorWorkflowInput {
        workflow_id: LOCAL_NODE_WORKFLOW_ID,
        steps: &step_checkpoints.steps,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        resource_refs: &resource_refs,
        replay_profile: "recorded",
    })?;
    let final_state_ref = canonical_hash(&record("operator-dogfood-final-state", vec![
        string(&state_root_ref),
        string(&shutdown_ref),
        string(&health_ref),
    ]))?;
    let report_value = dogfood_report_value(&DogfoodReportInput {
        workflow_value: &workflow_value,
        checkpoint_values: &step_checkpoints.checkpoints,
        gate_receipt_refs: &gate_receipt_refs,
        repro_bundle_refs: &repro_bundle_refs,
        final_state_ref: &final_state_ref,
        diagnostics: &[],
    })?;
    let report = parse_dogfood_report(&report_value)?;
    let validation_command_refs = vec![dogfood_ref("cargo-nextest-ci")?];
    let retention_gc_release_refs = vec![
        retention_gc.audit_ref.clone(),
        retention_gc.bundle_verify_ref.clone(),
        retention_gc.catalog_receipt_ref.clone(),
    ];
    let release_gate_value = if report.decision == "pass" {
        Some(release_gate_receipt_value(&ReleaseGateInput {
            report_value: &report_value,
            node_startup_ref: &startup_ref,
            node_shutdown_ref: &shutdown_ref,
            harness_gate_refs: &harness_gate_refs,
            catalog_query_refs: &catalog_query_refs,
            repro_verify_refs: &repro_verify_refs,
            retention_gc_refs: &retention_gc_release_refs,
            validation_command_refs: &validation_command_refs,
        })?)
    } else {
        None
    };
    let import_refs = import_dogfood_evidence(DogfoodEvidenceImportInput {
        ledger_root: &ledger_root,
        workflow_value: &workflow_value,
        step_values: &step_checkpoints.steps,
        checkpoint_values: &step_checkpoints.checkpoints,
        report_value: &report_value,
        release_gate_value: release_gate_value.as_ref(),
    })?;
    let release_gate_ref = release_gate_value.as_ref().map(canonical_hash).transpose()?;
    let StepCheckpointBuffers { steps, checkpoints } = step_checkpoints;
    Ok(LocalNodeDogfoodRun {
        decision: report.decision,
        workflow_ref: canonical_hash(&workflow_value)?,
        workflow_value,
        step_values: steps,
        checkpoint_values: checkpoints,
        report_ref: report.report_ref,
        report_value,
        release_gate_ref,
        release_gate_value,
        ledger_import_receipt_refs: import_refs,
    })
}

pub fn operator_dogfood_summary(value: &IOValue) -> Result<String> {
    if let Ok(report) = parse_dogfood_report(value) {
        return Ok(format!(
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
        return Ok(format!(
            "operator workflow ref={} id={} steps={} replay={} (summary is non-normative)",
            workflow.workflow_ref,
            workflow.workflow_id,
            workflow.steps.len(),
            workflow.replay_profile
        ));
    }
    if let Ok(checkpoint) = parse_operator_checkpoint(value) {
        return Ok(format!(
            "operator checkpoint ref={} workflow={} sequence={} step={} receipt={} (summary is non-normative)",
            checkpoint.checkpoint_ref,
            checkpoint.workflow_id,
            checkpoint.sequence,
            checkpoint.step_ref,
            checkpoint.receipt_ref.as_deref().unwrap_or("none")
        ));
    }
    if let Ok(receipt) = parse_release_gate_receipt(value) {
        return Ok(format!(
            "operator release gate receipt ref={} decision={} report={} checks={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.report_ref,
            receipt.checks.len()
        ));
    }
    if let Ok(evidence) = parse_nix_dogfood_evidence(value) {
        return Ok(format!(
            "operator Nix dogfood evidence ref={} output={} report={} release_gate={} nextest={} (summary is non-normative)",
            evidence.evidence_ref,
            evidence.output_path,
            evidence.report_ref,
            evidence.release_gate_ref,
            evidence.nextest_check_path
        ));
    }
    if let Ok(receipt) = parse_nix_dogfood_verify_receipt(value) {
        return Ok(format!(
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
        return Ok(format!(
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
        return Ok(format!(
            "operator release evidence bundle verify receipt ref={} decision={} bundle={} report={} release_gate={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.bundle_ref,
            receipt.report_ref,
            receipt.release_gate_ref,
            receipt.diagnostics.len()
        ));
    }
    if let Ok(receipt) = parse_release_promotion_gate_receipt(value) {
        return Ok(format!(
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
        return Ok(format!(
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
    if let Ok(manifest) = parse_release_export_manifest(value) {
        return Ok(format!(
            "operator release export manifest ref={} promotion_summary={} members={} (summary is non-normative)",
            manifest.manifest_ref,
            manifest.promotion_summary_ref,
            manifest.member_refs.len()
        ));
    }
    if let Ok(receipt) = parse_release_export_verify_receipt(value) {
        return Ok(format!(
            "operator release export verify receipt ref={} decision={} manifest={} promotion_summary={} diagnostics={} (summary is non-normative)",
            receipt.receipt_ref,
            receipt.decision,
            receipt.manifest_ref,
            receipt.promotion_summary_ref,
            receipt.diagnostics.len()
        ));
    }
    Err(MoltenError::invalid_harness("unsupported operator dogfood artifact for summary"))
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
    steps: Vec<IOValue>,
    checkpoints: Vec<IOValue>,
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
    let step_ref = canonical_hash(&step)?;
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
        workflow_ref: canonical_hash(&workflow_value)?,
        workflow_value,
        step_values: steps,
        checkpoint_values: checkpoints,
        report_ref: report.report_ref,
        report_value,
        release_gate_ref: None,
        release_gate_value: None,
        ledger_import_receipt_refs: Vec::new(),
    })
}

fn resolve_identity(state_root: &Path, policy_refs: &[String]) -> Result<node_identity::NodeIdentityResolution> {
    let mut config = node_identity::NodeIdentityConfig::new("node:dogfood-local", state_root.join("identity"));
    config.policy_refs = policy_refs.to_vec();
    node_identity::resolve_node_identity(&config)
}

fn start_node(
    identity: &node_identity::NodeIdentity,
    identity_receipt_ref: &str,
    policy_refs: &[String],
    capability_refs: &[String],
    resource_refs: &[String],
) -> Result<node_runtime::NodeRuntimeStart> {
    let adapter_bindings = node_runtime::REQUIRED_RUNTIME_ADAPTERS
        .iter()
        .map(|adapter| node_runtime::node_adapter_binding(adapter, &dogfood_ref(&format!("adapter:{adapter}"))?))
        .collect::<Result<Vec<_>>>()?;
    let state_root_ref = dogfood_ref("node-state-root")?;
    let effects_ref = dogfood_ref("effect-profile")?;
    let config_value = node_runtime::node_config_value(&node_runtime::ConfigValueInput {
        node_identity_ref: &identity.identity_ref,
        state_root_ref: &state_root_ref,
        adapters: &adapter_bindings,
        policy_refs,
        capability_refs,
        resource_refs,
        effect_profile_refs: &[effects_ref],
    })?;
    let source_gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = canonical_hash(&source_gate_value)?;
    node_runtime::start_node_runtime(&node_runtime::NodeRuntimeStartInput {
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
struct JobDogfoodRun {
    execution_request_ref: String,
    execution_receipt_ref: String,
    decision: String,
    diagnostics: Vec<String>,
    artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetentionDogfoodInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    ledger_root: &'a Path,
    registry_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetentionDogfoodRun {
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

fn run_job_stack(input: JobStackInput<'_>) -> Result<JobDogfoodRun> {
    let JobStackInput {
        state_root,
        source,
        target,
        policy_refs,
        capability_refs,
        resource_refs,
    } = input;
    let base = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
        kind: "schema".to_string(),
        payload: record("schema", vec![string("dogfood-job-base")]),
        schema_refs: vec![dogfood_ref("job-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-evidence")?],
        installer_ref: dogfood_ref("job-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let source_stage = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: job_dag::builtin_stage_operation_value("source")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let map_stage = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: job_dag::builtin_stage_operation_value("identity")?,
        schema_refs: vec![dogfood_ref("job-stage-schema")?],
        dependency_refs: vec![base.artifact_ref.clone()],
        effect_manifest_ref: None,
        policy_refs: policy_refs.to_vec(),
        evidence_refs: vec![dogfood_ref("job-stage-evidence")?],
        installer_ref: dogfood_ref("job-stage-installer")?,
        capability_refs: capability_refs.to_vec(),
    })?;
    let source_node = job_dag::job_node_value(job_dag::NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: Some(&source_stage.artifact_ref),
        input_ports: &[],
        output_ports: &["out".to_string()],
        config: record("source", vec![record("values", vec![sequence(vec![string("dogfood-job")])])]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let map_node = job_dag::job_node_value(job_dag::NodeValueInput {
        id: "map",
        kind: "map",
        stage_artifact_ref: Some(&map_stage.artifact_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: record("op", vec![string("identity")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?;
    let edge = job_dag::job_edge_value(job_dag::EdgeValueInput {
        from_node: "source",
        from_port: "out",
        to_node: "map",
        to_port: "in",
        schema_ref: None,
        partitioning: "single",
        materialization: "stream",
    })?;
    let dag = job_dag::job_dag_value(job_dag::DagValueInput {
        nodes: vec![source_node, map_node],
        edges: vec![edge],
        output_roots: &["map".to_string()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs,
        evidence_refs: std::slice::from_ref(&base.artifact_ref),
    })?;
    let installed_job = job_dag::install_job_dag(source, &dag)?;
    let mut sync_provenance = Vec::with_capacity(4);
    for artifact_ref in [
        base.artifact_ref.clone(),
        source_stage.artifact_ref.clone(),
        map_stage.artifact_ref.clone(),
        installed_job.artifact_ref.clone(),
    ] {
        sync_provenance.push_limited_value(
            crate::provenance::synthetic_reviewed_provenance_record(&artifact_ref)?,
            MAX_OPERATOR_REFS,
            "dogfood sync provenance",
        )?;
    }
    let sync_request = job_dag::job_sync_request_value(job_dag::SyncRequestValueInput {
        job_ref: &installed_job.job_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs,
        capability_refs,
        evidence_refs: &[dogfood_ref("job-sync-evidence")?],
    })?;
    let sync = job_dag::sync_loopback(job_dag::SyncLoopbackInput {
        source_registry: source,
        target_registry: target,
        request_value: &sync_request,
        provenance_values: &sync_provenance,
        build_verification_values: &[],
    })?;
    let sync_ref = canonical_hash(&sync.receipt_value)?;
    let authority_context_ref =
        install_job_execute_authority_context(target, &installed_job.job_ref, policy_refs, capability_refs)?;
    let source_gate_ref = install_clean_octet_gate(target, policy_refs, capability_refs)?;
    let admission_request = job_dag::job_admission_request_value(job_dag::AdmissionRequestValueInput {
        job_ref: &installed_job.job_ref,
        sync_ref: &sync_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs,
        capability_refs: std::slice::from_ref(&authority_context_ref),
        evidence_refs: &[sync_ref.clone(), source_gate_ref],
        resource_refs,
    })?;
    let admission = job_dag::admission_loopback(target, &admission_request)?;
    let admission_ref = canonical_hash(&admission.receipt_value)?;
    let execution_request = job_dag::job_execution_request_value(job_dag::ExecutionRequestValueInput {
        job_ref: &installed_job.job_ref,
        admission_ref: &admission_ref,
        stage_ids: &admission.plan.stage_order,
        target_peer: "peer:dogfood",
        storage_profile_ref: &dogfood_ref("job-storage-profile")?,
        cache_profile_ref: &dogfood_ref("job-cache-profile")?,
        chunk_profile_ref: &dogfood_ref("job-chunk-profile")?,
        policy_refs,
        capability_refs: std::slice::from_ref(&authority_context_ref),
        resource_refs,
    })?;
    let execution_request_ref = canonical_hash(&execution_request)?;
    let execution = job_dag::execution_loopback(job_dag::ExecutionLoopbackInput {
        target_registry: target,
        storage_root: &state_root.join("job-storage"),
        cache_root: &state_root.join("job-cache"),
        chunk_root: &state_root.join("job-chunks"),
        admission_receipt_value: &admission.receipt_value,
        request_value: &execution_request,
    })?;
    let mut artifact_refs = vec![
        installed_job.job_ref,
        sync_ref,
        admission_ref,
        authority_context_ref,
        execution_request_ref.clone(),
    ];
    if let Some(run) = execution.run.as_ref() {
        artifact_refs.extend(run.output_refs.iter().cloned());
    }
    Ok(JobDogfoodRun {
        execution_request_ref,
        execution_receipt_ref: execution.receipt_ref,
        decision: execution.decision,
        diagnostics: execution.diagnostics,
        artifact_refs,
    })
}

fn run_retention_gc_workflow(input: RetentionDogfoodInput<'_>) -> Result<RetentionDogfoodRun> {
    let object_ref = dogfood_ref("retention-object")?;
    let requester_ref = dogfood_ref("retention-requester")?;
    let peer_ref = dogfood_ref("retention-peer")?;
    let remote_ref = dogfood_ref("retention-remote-cache")?;
    let remote_refs = vec![remote_ref.clone()];
    let object_kind = "chunk";
    let retention_class = retention::CLASS_DURABLE_VALUE;
    let action = retention::ACTION_DELETE;
    let policy = store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: input.root,
        kind: retention::ADMISSION_KIND_POLICY,
        label: "policy",
        requester_ref: &requester_ref,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        remote_refs: &[],
    })?;
    let authority = store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: input.root,
        kind: retention::ADMISSION_KIND_AUTHORITY,
        label: "authority",
        requester_ref: &requester_ref,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        remote_refs: &[],
    })?;
    let support = store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: input.root,
        kind: retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
        label: "support",
        requester_ref: &requester_ref,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        remote_refs: &[],
    })?;
    let index = store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: input.root,
        kind: retention::ADMISSION_KIND_REFERENCE_INDEX,
        label: "index",
        requester_ref: &requester_ref,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        remote_refs: &[],
    })?;
    let remote_gc = store_retention_admission_fixture(RetentionAdmissionFixtureInput {
        root: input.root,
        kind: retention::ADMISSION_KIND_REMOTE_GC,
        label: "remote-gc",
        requester_ref: &requester_ref,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        remote_refs: &remote_refs,
    })?;
    let clearance_evidence = vec![support.admission_ref.clone()];
    let clearance =
        retention::store_retention_remote_gc_clearance(input.root, &retention::RetentionRemoteGcClearanceInput {
            decision: "pass",
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref: &remote_ref,
            policy_ref: &policy.admission_ref,
            authority_ref: &authority.admission_ref,
            evidence_refs: &clearance_evidence,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })?;
    let evidence = retention::DestructiveRetentionEvidence {
        requester_ref: Some(requester_ref),
        policy_refs: vec![policy.admission_ref.clone()],
        authority_refs: vec![authority.admission_ref.clone()],
        evidence_refs: vec![support.admission_ref.clone()],
        retained_refs: Vec::new(),
        remote_peer_refs: vec![peer_ref],
        remote_refs,
        reference_index_refs: vec![index.admission_ref.clone()],
        remote_gc_refs: vec![remote_gc.admission_ref.clone()],
        remote_clearance_refs: vec![clearance.clearance_ref.clone()],
        is_reference_index_complete: true,
    };
    let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
        root: input.root,
        subsystem: "ledger-gc",
        object_ref: &object_ref,
        object_kind,
        retention_class,
        action,
        evidence: &evidence,
    })?;
    let apply = retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
        root: input.root,
        plan_ref: &plan.plan_ref,
    })?;
    let execution = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
        root: input.root,
        subsystem: "ledger-gc",
        action,
        object_ref: &object_ref,
        object_kind,
        retention_class,
        apply_ref: Some(&apply.apply_ref),
    })?;
    let audit = retention::audit_retention_gc_execution(retention::RetentionGcAuditInput {
        root: input.root,
        execution_ref: &execution.execution_ref,
    })?;
    let explain = retention::explain_retention_candidate(retention::RetentionCandidateExplainInput {
        root: input.root,
        object_ref: &object_ref,
        object_kind: Some(object_kind),
        retention_class: Some(retention_class),
        action: Some(action),
        subsystem: Some("ledger-gc"),
    })?;
    let bundle = retention::export_retention_candidate_bundle(retention::RetentionCandidateBundleExportInput {
        root: input.root,
        explain_value: &explain.value,
        out: input.bundle_dir,
        profile: retention::RetentionCandidateBundleExportProfile::Public,
    })?;
    let profile_value =
        parse_text(&fs::read_to_string(input.bundle_dir.join("bundle-profile.preserves")).map_err(MoltenError::from)?)?;
    let profile = retention::parse_retention_candidate_bundle_profile(&profile_value)?;
    let verify = retention::verify_retention_candidate_bundle(retention::RetentionCandidateBundleVerifyInput {
        bundle_dir: input.bundle_dir,
    })?;
    let mut ledger_import_refs = Vec::new();
    for value in [
        &policy.value,
        &authority.value,
        &support.value,
        &index.value,
        &remote_gc.value,
        &clearance.value,
        &plan.value,
        &apply.value,
        &execution.value,
        &audit.value,
        &explain.value,
        &bundle.value,
        &profile.value,
        &verify.value,
    ] {
        let imported = ledger::import_artifact(input.ledger_root, value)?;
        ledger_import_refs.push_limited_value(
            canonical_hash(&imported.receipt_value)?,
            MAX_OPERATOR_REFS,
            "retention dogfood ledger imports",
        )?;
    }
    let mcp_request = catalog_mcp::mcp_request_value("search_retention_gc", vec![
        record("stage", vec![string("audit")]),
        record("object-ref", vec![string(&object_ref)]),
        record("subsystem", vec![string("ledger-gc")]),
    ])?;
    let mcp_call = catalog_mcp::call(input.registry_root, Some(input.ledger_root), &mcp_request)?;
    let catalog_receipt_ref = canonical_hash(&mcp_call.receipt_value)?;
    let mut bundle_diagnostics = Vec::new();
    append_dogfood_diagnostics(&mut bundle_diagnostics, "retention-bundle", &bundle.diagnostics)?;
    append_dogfood_diagnostics(&mut bundle_diagnostics, "retention-bundle-profile", &profile.diagnostics)?;
    append_dogfood_diagnostics(&mut bundle_diagnostics, "retention-bundle-verify", &verify.diagnostics)?;
    let mut artifact_refs = vec![
        policy.admission_ref,
        authority.admission_ref,
        support.admission_ref,
        index.admission_ref,
        remote_gc.admission_ref,
        clearance.clearance_ref,
        plan.plan_ref.clone(),
        apply.apply_ref.clone(),
        execution.execution_ref.clone(),
        audit.audit_ref.clone(),
        explain.explain_ref.clone(),
        bundle.bundle_ref.clone(),
        profile.profile_ref.clone(),
        verify.verify_ref.clone(),
        mcp_call.response_ref.clone(),
    ];
    artifact_refs.extend(ledger_import_refs);
    Ok(RetentionDogfoodRun {
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
    })
}

fn store_retention_admission_fixture(
    input: RetentionAdmissionFixtureInput<'_>,
) -> Result<retention::RetentionEvidenceAdmission> {
    let bound_refs = vec![dogfood_ref(&format!("retention-{}-bound", input.label))?];
    retention::store_retention_evidence_admission(input.root, &retention::RetentionEvidenceAdmissionInput {
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
    let context_value = authority::authority_context_value(authority::ContextValueInput {
        subject_ref: &subject_ref,
        capabilities: &[authority::AuthorityCapability {
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
    let context_ref = canonical_hash(&context_value)?;
    artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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
    let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let gate_ref = canonical_hash(&gate_value)?;
    artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
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
    let suite = parse_text(DOGFOOD_HARNESS_SUITE)?;
    let run = harness::run_suite_value(&suite)?;
    let gate = harness::gate_receipt_value(&harness::gate_check_value(&run.report_value)?);
    let gate_ref = canonical_hash(&gate)?;
    let bundle = harness::sealed_repro_bundle_value_with_command(&run.report_value, &[
        "molten".to_string(),
        "dogfood".to_string(),
        "local-node".to_string(),
    ])?;
    let bundle_ref = canonical_hash(&bundle)?;
    let verify = harness::repro_verify_receipt_value(&bundle)?;
    let verify_ref = canonical_hash(&verify)?;
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
    workflow_value: &'a IOValue,
    step_values: &'a [IOValue],
    checkpoint_values: &'a [IOValue],
    report_value: &'a IOValue,
    release_gate_value: Option<&'a IOValue>,
}

fn import_dogfood_evidence(input: DogfoodEvidenceImportInput<'_>) -> Result<Vec<String>> {
    let DogfoodEvidenceImportInput {
        ledger_root,
        workflow_value,
        step_values,
        checkpoint_values,
        report_value,
        release_gate_value,
    } = input;
    let mut imports = Vec::new();
    for value in step_values
        .iter()
        .chain(checkpoint_values.iter())
        .chain(std::iter::once(workflow_value))
        .chain(std::iter::once(report_value))
        .chain(release_gate_value)
    {
        let import = ledger::import_artifact(ledger_root, value)?;
        imports.push_limited_value(
            canonical_hash(&import.receipt_value)?,
            MAX_OPERATOR_REFS,
            "dogfood ledger import refs",
        )?;
    }
    Ok(imports)
}

fn service_lifecycle_pass(value: &IOValue) -> bool {
    crate::service_records::parse_service_lifecycle_receipt(value).is_ok_and(|receipt| receipt.decision == "pass")
}

fn dirty_state_reason(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_dir() {
        return Ok(Some("dogfood state root exists but is not a directory".to_string()));
    }
    let mut entries = fs::read_dir(path).map_err(MoltenError::from)?;
    if entries.next().transpose().map_err(MoltenError::from)?.is_some() {
        Ok(Some("dogfood local-node requires a clean empty state root".to_string()))
    } else {
        Ok(None)
    }
}

fn dogfood_ref(label: &str) -> Result<String> {
    canonical_hash(&record("operator-dogfood-ref", vec![string(label)]))
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
    if value.starts_with("blake3:") && value.len() > "blake3:".len() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} must be a blake3 ref")))
    }
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

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn step_receipts_sequence(receipts: &[(String, String)]) -> IOValue {
    sequence(
        receipts
            .iter()
            .map(|(name, reference)| record("step", vec![string(name), string(reference)]))
            .collect(),
    )
}

fn file_refs_sequence(refs: &[(String, String)]) -> IOValue {
    sequence(refs.iter().map(|(name, reference)| record("file", vec![string(name), string(reference)])).collect())
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "operator checks")?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, "operator checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
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

fn require_schema(value: &Value<IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let number = fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {label}")))
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_ref(&fields[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0])
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_ref(item.as_ref(), label))
        .collect()
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    record_iovalue_sequence(value, label)?
        .iter()
        .map(|item| required_string(item.as_ref(), label))
        .collect()
}

fn record_iovalue_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_OPERATOR_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited_value(value_to_iovalue(item), MAX_OPERATOR_REFS, label)?;
    }
    Ok(values)
}

fn record_step_receipts(value: &Value<IOValue>, label: &str) -> Result<Vec<(String, String)>> {
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

fn record_file_refs(value: &Value<IOValue>, label: &str) -> Result<Vec<(String, String)>> {
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

fn member_ref(value: &Value<IOValue>, expected_name: &str) -> Result<String> {
    record_file_refs(value, "members")?
        .into_iter()
        .find_map(|(name, reference)| (name == expected_name).then_some(reference))
        .ok_or_else(|| MoltenError::invalid_harness(format!("release evidence bundle missing member {expected_name}")))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| MoltenError::invalid_harness(format!("{field} overflows u64: {error}")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::evidence::SignReceiptInput;
    use crate::evidence::SignedReceiptKeyInput;
    use crate::evidence::SignedReceiptKeyRevocationInput;
    use crate::evidence::parse_signed_receipt_key;
    use crate::evidence::parse_signed_receipt_key_revocation;
    use crate::evidence::sign_receipt;
    use crate::evidence::signed_receipt_key_revocation_value;
    use crate::evidence::signed_receipt_key_value;
    use crate::preserves_rail::to_text;

    #[test]
    fn local_node_dogfood_runs_and_gates_release() {
        let root = temp_dir("operator-dogfood-pass");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dogfood run");
        assert_eq!(run.decision, "pass", "{}", to_text(&run.report_value).expect("report text"));
        assert!(run.release_gate_ref.as_deref().is_some_and(|reference| reference.starts_with("blake3:")));
        assert_eq!(crate::ledger::artifact_kind(&run.workflow_value), "operator-workflow");
        assert_eq!(crate::ledger::artifact_kind(&run.report_value), "dogfood-report");
        assert_eq!(
            crate::ledger::artifact_kind(run.release_gate_value.as_ref().expect("release gate")),
            "release-gate-receipt"
        );
        let entries = ledger::list_artifacts(&root.join("ledger")).expect("ledger entries");
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
        let release_text = to_text(run.release_gate_value.as_ref().expect("release gate text")).expect("release text");
        assert!(release_text.contains("retention-gc-review-bound"));
        assert!(release_text.contains("retention-gc-is-evidence-only"));
        assert!(operator_dogfood_summary(&run.report_value).expect("summary").contains("decision=pass"));
    }

    #[test]
    fn nix_dogfood_release_evidence_verifies_and_denies_stale_refs() {
        let root = temp_dir("operator-dogfood-nix-evidence");
        let state_root = root.join("state");
        let output_root = root.join("nix-output");
        fs::create_dir_all(&output_root).expect("create nix output");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput {
            state_root: &state_root,
        })
        .expect("dogfood run");
        fs::write(output_root.join("dogfood-report.preserves"), to_text(&run.report_value).expect("report text"))
            .expect("write report");
        fs::write(
            output_root.join("release-gate.preserves"),
            to_text(run.release_gate_value.as_ref().expect("release gate")).expect("release text"),
        )
        .expect("write release gate");
        fs::write(
            output_root.join("dogfood-summary.txt"),
            format!(
                "dogfood local-node decision=pass report={} release-gate={}\n",
                run.report_ref,
                run.release_gate_ref.as_deref().expect("release ref")
            ),
        )
        .expect("write summary");
        fs::write(output_root.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n")
            .expect("write nextest marker");
        let evidence = nix_dogfood_release_evidence_value(&NixDogfoodEvidenceInput {
            output_path: &output_root,
        })
        .expect("nix evidence");
        let parsed = parse_nix_dogfood_evidence(&evidence).expect("parse nix evidence");
        assert_eq!(crate::ledger::artifact_kind(&evidence), "nix-dogfood-release-evidence");
        assert_eq!(parsed.release_gate_ref, run.release_gate_ref.expect("release ref"));
        let receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &output_root,
            evidence_value: &evidence,
        })
        .expect("verify nix evidence");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&receipt.value), "nix-dogfood-release-verify-receipt");
        fs::write(output_root.join("nix-dogfood-evidence.preserves"), to_text(&evidence).expect("evidence text"))
            .expect("write evidence");
        fs::write(output_root.join("nix-dogfood-verify.preserves"), to_text(&receipt.value).expect("verify text"))
            .expect("write verify");
        let bundle = release_evidence_bundle_value(&ReleaseEvidenceBundleInput {
            output_path: &output_root,
        })
        .expect("release bundle");
        let parsed_bundle = parse_release_evidence_bundle(&bundle).expect("parse release bundle");
        assert_eq!(crate::ledger::artifact_kind(&bundle), "release-evidence-bundle");
        assert_eq!(parsed_bundle.report_ref, parsed.report_ref);
        let bundle_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &bundle,
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
        .expect("verify release bundle");
        assert_eq!(bundle_verify.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&bundle_verify.value), "release-evidence-bundle-verify-receipt");
        let signed_members = vec![
            sign_receipt(&SignReceiptInput {
                receipt: &run.report_value,
                signer: "release-signer",
                purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
                trust_root: "release-root",
                key: "release-key",
                parents: &[],
            })
            .expect("sign report"),
            sign_receipt(&SignReceiptInput {
                receipt: run.release_gate_value.as_ref().expect("release gate"),
                signer: "release-signer",
                purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
                trust_root: "release-root",
                key: "release-key",
                parents: &[],
            })
            .expect("sign release gate"),
            sign_receipt(&SignReceiptInput {
                receipt: &evidence,
                signer: "release-signer",
                purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
                trust_root: "release-root",
                key: "release-key",
                parents: &[],
            })
            .expect("sign Nix evidence"),
            sign_receipt(&SignReceiptInput {
                receipt: &receipt.value,
                signer: "release-signer",
                purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
                trust_root: "release-root",
                key: "release-key",
                parents: &[],
            })
            .expect("sign Nix verify"),
        ];
        let signed_bundle_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &bundle,
            signed_member_values: &signed_members,
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys: &[],
            signed_key_revocations: &[],
            signed_key_ref: None,
            signed_key_id: None,
            signed_signer: Some("release-signer"),
            is_signed_members_required: true,
        })
        .expect("verify signed release bundle");
        assert_eq!(signed_bundle_verify.decision, "pass");
        let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
            key_id: "release-key-1",
            signer: "release-signer",
            trust_root: "release-root",
            key: "release-key",
            generation: 1,
            predecessor_ref: None,
        })
        .expect("signed key value");
        let key = parse_signed_receipt_key(&key_value).expect("parse signed key");
        let promotion = release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: &output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: &[],
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("promotion receipt");
        assert_eq!(promotion.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&promotion.value), "release-promotion-gate-receipt");
        let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
            key: &key,
            reason: "test-revoked",
            superseded_by: None,
        })
        .expect("revocation value");
        let revocation = parse_signed_receipt_key_revocation(&revocation_value).expect("parse revocation");
        let revoked_promotion = release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: &output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: std::slice::from_ref(&revocation),
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("revoked promotion receipt");
        assert_eq!(revoked_promotion.decision, "deny");
        assert!(
            revoked_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("revoked") || diagnostic.contains("stale"))
        );
        let missing_source_promotion = release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: &output_root,
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "",
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: &[],
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("missing source promotion receipt");
        assert_eq!(missing_source_promotion.decision, "deny");
        assert!(missing_source_promotion.diagnostics.iter().any(|diagnostic| diagnostic.contains("source evidence")));
        let stale_output_promotion = release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: &output_root.join("stale-output"),
            bundle_verify_value: &signed_bundle_verify.value,
            source_evidence: "source:working-tree-reviewed",
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: &[],
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("stale output promotion receipt");
        assert_eq!(stale_output_promotion.decision, "deny");
        assert!(
            stale_output_promotion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("output-path-ref mismatch"))
        );
        let missing_signed_member_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &bundle,
            signed_member_values: &[],
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: &[],
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
            signed_signer: Some("release-signer"),
            is_signed_members_required: true,
        })
        .expect("missing signed member verify receipt");
        assert_eq!(missing_signed_member_verify.decision, "deny");
        let denied_bundle_promotion = release_promotion_gate_receipt_value(&ReleasePromotionGateInput {
            output_path: &output_root,
            bundle_verify_value: &missing_signed_member_verify.value,
            source_evidence: "source:working-tree-reviewed",
            octet_evidence: "octet:clean",
            cairn_evidence: "cairn:strict-validate",
            signed_keys: std::slice::from_ref(&key),
            signed_key_revocations: &[],
            signed_trust_root: "release-root",
            signed_signer: Some("release-signer"),
            signed_key_ref: Some(&key.key_ref),
            signed_key_id: Some("release-key-1"),
        })
        .expect("denied bundle promotion receipt");
        assert_eq!(denied_bundle_promotion.decision, "deny");
        assert!(denied_bundle_promotion.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));
        let wrong_signer_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &bundle,
            signed_member_values: &signed_members,
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys: &[],
            signed_key_revocations: &[],
            signed_key_ref: None,
            signed_key_id: None,
            signed_signer: Some("wrong-signer"),
            is_signed_members_required: true,
        })
        .expect("verify wrong signer release bundle");
        assert_eq!(wrong_signer_verify.decision, "deny");
        assert!(wrong_signer_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("signer")));
        let missing_signed_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &bundle,
            signed_member_values: &signed_members[..1],
            signed_purpose: RELEASE_EVIDENCE_SIGNING_PURPOSE,
            signed_trust_root: "release-root",
            signed_key: "release-key",
            signed_keys: &[],
            signed_key_revocations: &[],
            signed_key_ref: None,
            signed_key_id: None,
            signed_signer: Some("release-signer"),
            is_signed_members_required: true,
        })
        .expect("verify missing signed member release bundle");
        assert_eq!(missing_signed_verify.decision, "deny");
        assert!(
            missing_signed_verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing signed member receipt"))
        );
        let stale_bundle_ref = dogfood_ref("stale-bundle-summary").expect("stale bundle ref");
        let stale_bundle_text =
            to_text(&bundle).expect("bundle text").replace(&parsed_bundle.summary_ref, &stale_bundle_ref);
        let stale_bundle = parse_text(&stale_bundle_text).expect("stale bundle parse");
        let stale_bundle_verify = verify_release_evidence_bundle(&ReleaseEvidenceBundleVerifyInput {
            output_path: &output_root,
            bundle_value: &stale_bundle,
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
        .expect("verify stale bundle");
        assert_eq!(stale_bundle_verify.decision, "deny");
        assert!(stale_bundle_verify.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
        let stale_ref = dogfood_ref("stale-summary").expect("stale ref");
        let stale_text = to_text(&evidence).expect("evidence text").replace(&parsed.summary_ref, &stale_ref);
        let stale_evidence = parse_text(&stale_text).expect("stale evidence parse");
        let stale_receipt = verify_nix_dogfood_evidence(&NixDogfoodVerifyInput {
            output_path: &output_root,
            evidence_value: &stale_evidence,
        })
        .expect("verify stale evidence");
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("summary-ref mismatch")));
    }

    #[test]
    fn missing_receipt_and_non_replayable_mandatory_steps_deny_report() {
        let request_ref = dogfood_ref("request").expect("request ref");
        let missing_step = operator_step_value(&OperatorStepInput {
            name: "install-artifact",
            request_ref: Some(&request_ref),
            receipt_ref: None,
            decision: "pass",
            replay_status: "deterministic",
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("missing step");
        let live_receipt = dogfood_ref("live-receipt").expect("live receipt");
        let live_step = operator_step_value(&OperatorStepInput {
            name: "live-diagnostic",
            request_ref: Some(&request_ref),
            receipt_ref: Some(&live_receipt),
            decision: "pass",
            replay_status: "non-replayable",
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("live step");
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
        let report = dogfood_report_value(&DogfoodReportInput {
            workflow_value: &workflow,
            checkpoint_values: &[checkpoint],
            gate_receipt_refs: &[dogfood_ref("gate").expect("gate")],
            repro_bundle_refs: &[dogfood_ref("repro").expect("repro")],
            final_state_ref: &dogfood_ref("final-state").expect("final state"),
            diagnostics: &[],
        })
        .expect("report");
        let parsed = parse_dogfood_report(&report).expect("parse report");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("lacks canonical receipt")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-release replay status")));
        assert!(
            release_gate_receipt_value(&ReleaseGateInput {
                report_value: &report,
                node_startup_ref: &dogfood_ref("startup").expect("startup"),
                node_shutdown_ref: &dogfood_ref("shutdown").expect("shutdown"),
                harness_gate_refs: &[dogfood_ref("harness-gate").expect("harness gate")],
                catalog_query_refs: &[dogfood_ref("catalog").expect("catalog")],
                repro_verify_refs: &[dogfood_ref("verify").expect("verify")],
                retention_gc_refs: &[dogfood_ref("retention-gc").expect("retention gc")],
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
        fs::write(root.join("leftover"), "dirty").expect("write dirty marker");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dirty report");
        assert_eq!(run.decision, "deny");
        assert!(run.release_gate_value.is_none());
        let report = parse_dogfood_report(&run.report_value).expect("parse dirty report");
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.contains("clean empty state root")));
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
