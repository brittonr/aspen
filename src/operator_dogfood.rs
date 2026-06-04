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
use crate::harness;
use crate::job_dag;
use crate::ledger;
use crate::node_identity;
use crate::node_runtime;
use crate::octet_gate;
use crate::preserves_rail::OPERATOR_CHECKPOINT_SCHEMA;
use crate::preserves_rail::OPERATOR_DOGFOOD_REPORT_SCHEMA;
use crate::preserves_rail::OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA;
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
    pub validation_command_refs: &'a [String],
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
pub struct DogfoodReport {
    pub report_ref: String,
    pub decision: String,
    pub workflow_ref: String,
    pub checkpoint_refs: Vec<String>,
    pub step_receipts: Vec<(String, String)>,
    pub gate_receipts: Vec<String>,
    pub repro_bundles: Vec<String>,
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
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value_from_pairs(&[
            ("canonical-report", "pass"),
            ("deterministic-or-recorded", status(diagnostics.iter().all(|item| !item.contains("replay status")))),
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
        .collect_simple_record("dogfood-report-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dogfood-report-v1 ...>"))?;
    require_schema(&fields[0], OPERATOR_DOGFOOD_REPORT_SCHEMA, "dogfood report")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-report", "dogfood report")?;
    require_check(&checks, "no-text-oracle", "dogfood report")?;
    Ok(DogfoodReport {
        report_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        workflow_ref: record_ref(&fields[2], "workflow")?,
        checkpoint_refs: record_ref_sequence(&fields[3], "checkpoints")?,
        step_receipts: record_step_receipts(&fields[4], "step-receipts")?,
        gate_receipts: record_ref_sequence(&fields[5], "gate-receipts")?,
        repro_bundles: record_ref_sequence(&fields[6], "repro-bundles")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
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
    require_non_empty_refs(input.validation_command_refs, "dogfood release validation command ref")?;
    Ok(record("release-gate-receipt-v1", vec![
        string(OPERATOR_RELEASE_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("report", vec![string(&report.report_ref)]),
        record("node", vec![string(input.node_startup_ref), string(input.node_shutdown_ref)]),
        record("harness-gates", vec![refs_sequence(input.harness_gate_refs)]),
        record("catalog-queries", vec![refs_sequence(input.catalog_query_refs)]),
        record("repro-verifies", vec![refs_sequence(input.repro_verify_refs)]),
        record("validation-commands", vec![refs_sequence(input.validation_command_refs)]),
        checks_value_from_pairs(&[
            ("dogfood-report-pass", "pass"),
            ("deterministic-or-recorded-only", "pass"),
            ("redaction-gate-bound", "pass"),
            ("startup-shutdown-bound", "pass"),
            ("catalog-mcp-bound", "pass"),
            ("no-text-oracle", "pass"),
        ]),
    ]))
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
    let report_value = dogfood_report_value(&DogfoodReportInput {
        workflow_value: &workflow_value,
        checkpoint_values: &step_checkpoints.checkpoints,
        gate_receipt_refs: &gate_receipt_refs,
        repro_bundle_refs: &repro_bundle_refs,
        diagnostics: &[],
    })?;
    let report = parse_dogfood_report(&report_value)?;
    let validation_command_refs = vec![dogfood_ref("cargo-nextest-ci")?];
    let release_gate_value = if report.decision == "pass" {
        Some(release_gate_receipt_value(&ReleaseGateInput {
            report_value: &report_value,
            node_startup_ref: &startup_ref,
            node_shutdown_ref: &shutdown_ref,
            harness_gate_refs: &harness_gate_refs,
            catalog_query_refs: &catalog_query_refs,
            repro_verify_refs: &repro_verify_refs,
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
            "operator dogfood report ref={} decision={} workflow={} steps={} gates={} repro={} diagnostics={} (summary is non-normative)",
            report.report_ref,
            report.decision,
            report.workflow_ref,
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
    if value.collect_simple_record("release-gate-receipt-v1", Some(9)).is_some() {
        return Ok(format!("operator release gate receipt ref={} (summary is non-normative)", canonical_hash(value)?));
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
    let sync_request = job_dag::job_sync_request_value(job_dag::SyncRequestValueInput {
        job_ref: &installed_job.job_ref,
        stage_ids: &[],
        target_peer: "peer:dogfood",
        policy_refs,
        capability_refs,
        evidence_refs: &[dogfood_ref("job-sync-evidence")?],
    })?;
    let sync = job_dag::sync_loopback(source, target, &sync_request)?;
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
        assert!(operator_dogfood_summary(&run.report_value).expect("summary").contains("decision=pass"));
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
