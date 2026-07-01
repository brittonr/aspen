use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use preserves::IOValue;
use preserves::Value;

use crate::chunk_store;
use crate::error::Result;
use crate::preserves_rail;

const DEFAULT_ARTIFACT_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const DEFAULT_CLOSURE_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const DEFAULT_INITIAL_STATE_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const DEFAULT_SCHEMA_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const DEFAULT_POLICY_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const DEFAULT_CAPABILITY_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const DEFAULT_REVOCATION_REF: &str = "blake3:7777777777777777777777777777777777777777777777777777777777777777";
const DEFAULT_HANDLER_PROFILE_REF: &str = "blake3:8888888888888888888888888888888888888888888888888888888888888888";
const DEFAULT_SEED_REF: &str = "blake3:9999999999999999999999999999999999999999999999999999999999999999";
const DEFAULT_RUNTIME_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DEFAULT_TOOL_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MAX_REPLAY_ROLLUP_INPUTS: usize = 1024;
const MAX_REPLAY_INDEX_INPUTS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayDivergenceKind {
    None,
    Identity,
    Scheduler,
    Input,
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    Action,
    Receipt,
    Output,
    StateHash,
    LiveEffect,
}

impl ReplayDivergenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ReplayDivergenceKind::None => "none",
            ReplayDivergenceKind::Identity => "identity",
            ReplayDivergenceKind::Scheduler => "scheduler",
            ReplayDivergenceKind::Input => "input",
            ReplayDivergenceKind::EffectRequest => "effect-request",
            ReplayDivergenceKind::EffectResponse => "effect-response",
            ReplayDivergenceKind::PolicyDecision => "policy-decision",
            ReplayDivergenceKind::Action => "action",
            ReplayDivergenceKind::Receipt => "receipt",
            ReplayDivergenceKind::Output => "output",
            ReplayDivergenceKind::StateHash => "state-hash",
            ReplayDivergenceKind::LiveEffect => "live-effect",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayFixtureVariant {
    Baseline,
    ChangedIdentity,
    ChangedScheduler,
    ChangedInput,
    ChangedEffectRequest,
    ChangedEffectResponse,
    ChangedPolicyDecision,
    ChangedAction,
    ChangedReceipt,
    ChangedOutput,
    ChangedStateHash,
    MissingRecordedEffect,
}

#[derive(Clone, Debug)]
pub struct ReplayFixtureRecord {
    pub value: IOValue,
    pub record_ref: String,
    pub identity_ref: String,
    pub effect_log_ref: String,
    pub final_state_ref: String,
    pub output_ref: String,
}

#[derive(Clone, Debug)]
pub struct ReplaySnapshotManifestBundle {
    pub value: IOValue,
    pub bundle_ref: String,
    pub effect_log_manifest_ref: String,
    pub turn_journal_manifest_ref: String,
    pub snapshot_manifest_ref: String,
    pub first_divergence_manifest_ref: Option<String>,
    pub debug_range_receipt_ref: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReplayVerifyReceipt {
    pub value: IOValue,
    pub receipt_ref: String,
    pub decision: &'static str,
    pub divergence: ReplayDivergenceKind,
    pub first_divergence: Option<IOValue>,
}

#[derive(Clone, Debug)]
pub struct ReplayRollupInput {
    pub expected_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Clone, Debug)]
pub struct ReplayRollupReceipt {
    pub value: IOValue,
    pub rollup_ref: String,
    pub decision: String,
    pub total_count: u64,
    pub pass_count: u64,
    pub deny_count: u64,
}

#[derive(Clone, Debug)]
pub struct ReplayIndexInput {
    pub expected_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Clone, Debug)]
pub struct ReplayIndexReceipt {
    pub value: IOValue,
    pub index_ref: String,
    pub decision: String,
    pub total_count: u64,
    pub pass_count: u64,
    pub deny_count: u64,
    pub raw_receipt_count: u64,
    pub rollup_count: u64,
}

#[derive(Clone, Debug)]
pub struct ChaosScheduleInput {
    pub seed_ref: String,
    pub schedule_position: u64,
    pub event_ref: String,
    pub fault_kind: String,
    pub intensity_percent: u64,
}

#[derive(Clone, Debug)]
pub struct ChaosScheduleReceipt {
    pub value: IOValue,
    pub schedule_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
pub struct TracePrivacyInput {
    pub trace_ref: String,
    pub snapshot_ref: String,
    pub requester_ref: String,
    pub policy_ref: String,
    pub has_export_authority: bool,
    pub contains_sensitive_refs: bool,
}

#[derive(Clone, Debug)]
pub struct TracePrivacyReceipt {
    pub value: IOValue,
    pub receipt_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
pub struct DeterministicIntegrationInput {
    pub integration_kind: String,
    pub handler_profile_ref: String,
    pub effect_log_ref: String,
    pub snapshot_ref: String,
    pub gate_ref: String,
    pub admitted_live_effects: bool,
}

#[derive(Clone, Debug)]
pub struct DeterministicIntegrationReceipt {
    pub value: IOValue,
    pub receipt_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
struct ParsedReplayVerify {
    receipt_ref: String,
    decision: String,
    divergence: String,
    first_divergence_ref: Option<String>,
    report_refs: Vec<String>,
    final_state_refs: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedReplayRollup {
    rollup_ref: String,
    decision: String,
    total_count: u64,
    pass_count: u64,
    deny_count: u64,
    receipt_refs: Vec<String>,
    divergence_counts: BTreeMap<String, u64>,
    first_divergence_refs: Vec<String>,
}

#[derive(Clone)]
struct ReplayRunParts {
    identity: IOValue,
    identity_ref: String,
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
    after_state_ref: String,
    turn_journal: IOValue,
    turn_journal_ref: String,
    effect_log: IOValue,
    effect_log_ref: String,
}

pub fn record_fixture_value() -> Result<ReplayFixtureRecord> {
    let parts = run_parts(ReplayFixtureVariant::Baseline)?;
    let value = preserves_rail::record("deterministic-fixture-record-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_FIXTURE_RECORD_SCHEMA),
        preserves_rail::record("identity-ref", vec![preserves_rail::string(&parts.identity_ref)]),
        parts.identity,
        preserves_rail::record("effect-log-ref", vec![preserves_rail::string(&parts.effect_log_ref)]),
        parts.effect_log,
        preserves_rail::sequence(vec![parts.turn_journal]),
        preserves_rail::record("output-ref", vec![preserves_rail::string(&parts.output_ref)]),
        preserves_rail::record("final-state-ref", vec![preserves_rail::string(&parts.after_state_ref)]),
        preserves_rail::sequence(vec![
            preserves_rail::string("recorded-responses-bound"),
            preserves_rail::string("canonical-journal-order"),
            preserves_rail::string("no-ambient-observations"),
        ]),
    ]);
    let record_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ReplayFixtureRecord {
        value,
        record_ref,
        identity_ref: parts.identity_ref,
        effect_log_ref: parts.effect_log_ref,
        final_state_ref: parts.after_state_ref,
        output_ref: parts.output_ref,
    })
}

pub fn replay_snapshot_manifest_bundle(
    chunk_root: &Path,
    variant: ReplayFixtureVariant,
) -> Result<ReplaySnapshotManifestBundle> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    let snapshot = preserves_rail::record("deterministic-replay-snapshot-v1", vec![
        preserves_rail::string("molten.deterministic-replay.snapshot.v1"),
        preserves_rail::record("identity-ref", vec![preserves_rail::string(&expected.identity_ref)]),
        preserves_rail::record("final-state-ref", vec![preserves_rail::string(&expected.after_state_ref)]),
        preserves_rail::record("turn-journal-ref", vec![preserves_rail::string(&expected.turn_journal_ref)]),
        preserves_rail::record("effect-log-ref", vec![preserves_rail::string(&expected.effect_log_ref)]),
        preserves_rail::sequence(vec![
            preserves_rail::string("manifest-backed"),
            preserves_rail::string("partial-debug-fetch"),
        ]),
    ]);
    let effect_log_manifest_ref = store_replay_manifest(chunk_root, "replay-effect-log", &expected.effect_log)?;
    let turn_journal_manifest_ref = store_replay_manifest(chunk_root, "replay-turn-journal", &expected.turn_journal)?;
    let snapshot_manifest_ref = store_replay_manifest(chunk_root, "replay-snapshot", &snapshot)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let (first_divergence_manifest_ref, debug_range_receipt_ref) = if divergence == ReplayDivergenceKind::None {
        (None, None)
    } else {
        let divergence_value = first_divergence_value(divergence, &expected, &actual)?;
        let manifest_ref = store_replay_manifest(chunk_root, "replay-first-divergence", &divergence_value)?;
        let range = chunk_store::range_read(chunk_root, &manifest_ref, 0, 32)?;
        (Some(manifest_ref), Some(preserves_rail::canonical_hash(&range.receipt_value)?))
    };
    let value = preserves_rail::record("deterministic-replay-snapshot-manifests-v1", vec![
        preserves_rail::string("molten.deterministic-replay.snapshot-manifests.v1"),
        preserves_rail::record("effect-log-manifest-ref", vec![preserves_rail::string(&effect_log_manifest_ref)]),
        preserves_rail::record("turn-journal-manifest-ref", vec![preserves_rail::string(&turn_journal_manifest_ref)]),
        preserves_rail::record("snapshot-manifest-ref", vec![preserves_rail::string(&snapshot_manifest_ref)]),
        preserves_rail::record("first-divergence-manifest-ref", vec![optional_ref_value(
            first_divergence_manifest_ref.as_deref(),
        )]),
        preserves_rail::record("debug-range-receipt-ref", vec![optional_ref_value(debug_range_receipt_ref.as_deref())]),
        preserves_rail::sequence(vec![
            preserves_rail::string("manifest-backed-replay"),
            preserves_rail::string("verified-before-load"),
            preserves_rail::string("partial-divergence-debug-fetch"),
        ]),
    ]);
    let bundle_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ReplaySnapshotManifestBundle {
        value,
        bundle_ref,
        effect_log_manifest_ref,
        turn_journal_manifest_ref,
        snapshot_manifest_ref,
        first_divergence_manifest_ref,
        debug_range_receipt_ref,
    })
}

fn store_replay_manifest(chunk_root: &Path, object_kind: &str, value: &IOValue) -> Result<String> {
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    Ok(chunk_store::put_bytes(chunk_root, object_kind, &bytes, chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE)?.manifest_ref)
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => preserves_rail::record("some", vec![preserves_rail::string(value)]),
        None => preserves_rail::record("none", Vec::new()),
    }
}

pub fn verify_fixture_value(variant: ReplayFixtureVariant) -> Result<ReplayVerifyReceipt> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let first_divergence = if divergence == ReplayDivergenceKind::None {
        None
    } else {
        Some(first_divergence_value(divergence, &expected, &actual)?)
    };
    let first_divergence_ref = match &first_divergence {
        Some(value) => preserves_rail::canonical_hash(value)?,
        None => "none".to_string(),
    };
    let decision = if divergence == ReplayDivergenceKind::None {
        "pass"
    } else {
        "deny"
    };
    let value = preserves_rail::record("deterministic-replay-verify-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        preserves_rail::string(decision),
        preserves_rail::record("expected-identity-ref", vec![preserves_rail::string(&expected.identity_ref)]),
        preserves_rail::record("actual-identity-ref", vec![preserves_rail::string(&actual.identity_ref)]),
        preserves_rail::record("expected-effect-log-ref", vec![preserves_rail::string(&expected.effect_log_ref)]),
        preserves_rail::record("actual-effect-log-ref", vec![preserves_rail::string(&actual.effect_log_ref)]),
        preserves_rail::record("expected-output-ref", vec![preserves_rail::string(&expected.output_ref)]),
        preserves_rail::record("actual-output-ref", vec![preserves_rail::string(&actual.output_ref)]),
        preserves_rail::record("expected-final-state-ref", vec![preserves_rail::string(&expected.after_state_ref)]),
        preserves_rail::record("actual-final-state-ref", vec![preserves_rail::string(&actual.after_state_ref)]),
        preserves_rail::record("divergence", vec![preserves_rail::string(divergence.as_str())]),
        preserves_rail::record("first-divergence-ref", vec![preserves_rail::string(&first_divergence_ref)]),
        preserves_rail::sequence(verify_checks(decision, divergence)),
    ]);
    let receipt_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ReplayVerifyReceipt {
        value,
        receipt_ref,
        decision,
        divergence,
        first_divergence,
    })
}

pub fn rollup_replay_receipts(inputs: &[ReplayRollupInput]) -> Result<ReplayRollupReceipt> {
    if inputs.len() > MAX_REPLAY_ROLLUP_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "replay rollup input count exceeds {MAX_REPLAY_ROLLUP_INPUTS}"
        )));
    }
    let mut diagnostics = Vec::with_capacity(inputs.len());
    let mut parsed_receipts = Vec::with_capacity(inputs.len());
    for input in inputs {
        let actual_ref = preserves_rail::canonical_hash(&input.value)?;
        if let Some(expected_ref) = input.expected_ref.as_deref() {
            preserves_rail::validate_content_ref(expected_ref)?;
            if expected_ref != actual_ref {
                diagnostics.push(format!("replay receipt ref mismatch expected={expected_ref} actual={actual_ref}"));
                continue;
            }
        }
        match parse_replay_verify_receipt(&input.value, &actual_ref) {
            Ok(parsed) => parsed_receipts.push(parsed),
            Err(error) => diagnostics.push(format!("replay receipt {actual_ref} is invalid: {error}")),
        }
    }
    let mut receipt_refs = BTreeSet::new();
    let mut first_divergence_refs = BTreeSet::new();
    let mut divergence_counts = BTreeMap::<String, u64>::new();
    let mut pass_count = 0_u64;
    let mut deny_count = 0_u64;
    for parsed in &parsed_receipts {
        receipt_refs.insert(parsed.receipt_ref.clone());
        *divergence_counts.entry(parsed.divergence.clone()).or_insert(0) += 1;
        if parsed.decision == "pass" {
            pass_count += 1;
        } else {
            deny_count += 1;
        }
        if let Some(reference) = &parsed.first_divergence_ref {
            first_divergence_refs.insert(reference.clone());
        }
    }
    let total_count = parsed_receipts.len() as u64;
    let decision = if diagnostics.is_empty() && deny_count == 0 {
        "pass"
    } else {
        "deny"
    };
    let value = preserves_rail::record("deterministic-replay-rollup-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_REPLAY_ROLLUP_SCHEMA),
        preserves_rail::record("decision", vec![preserves_rail::string(decision)]),
        preserves_rail::record("total-count", vec![preserves_rail::u64_value(total_count)]),
        preserves_rail::record("pass-count", vec![preserves_rail::u64_value(pass_count)]),
        preserves_rail::record("deny-count", vec![preserves_rail::u64_value(deny_count)]),
        preserves_rail::record("receipt-refs", vec![refs_value(&receipt_refs)]),
        preserves_rail::record("divergence-counts", vec![divergence_counts_value(&divergence_counts)]),
        preserves_rail::record("first-divergence-refs", vec![refs_value(&first_divergence_refs)]),
        preserves_rail::record("diagnostics", vec![preserves_rail::sequence(
            diagnostics.iter().map(preserves_rail::string).collect(),
        )]),
        preserves_rail::sequence(rollup_checks(decision, diagnostics.is_empty())),
    ]);
    let rollup_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ReplayRollupReceipt {
        value,
        rollup_ref,
        decision: decision.to_string(),
        total_count,
        pass_count,
        deny_count,
    })
}

pub fn index_replay_evidence(inputs: &[ReplayIndexInput]) -> Result<ReplayIndexReceipt> {
    if inputs.len() > MAX_REPLAY_INDEX_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "replay index input count exceeds {MAX_REPLAY_INDEX_INPUTS}"
        )));
    }
    let parsed = collect_index_inputs(inputs)?;
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(rollup_anomalies(&parsed.rollups));
    let summary = summarize_index_inputs(&parsed.receipts, &parsed.rollups);
    let decision = if diagnostics.is_empty() && summary.deny_count == 0 {
        "pass"
    } else {
        "deny"
    };
    let value = index_value(decision, &diagnostics, &summary);
    let index_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ReplayIndexReceipt {
        value,
        index_ref,
        decision: decision.to_string(),
        total_count: summary.total_count,
        pass_count: summary.pass_count,
        deny_count: summary.deny_count,
        raw_receipt_count: summary.raw_receipt_count,
        rollup_count: summary.rollup_count,
    })
}

struct ParsedInputs {
    diagnostics: Vec<String>,
    receipts: Vec<ParsedReplayVerify>,
    rollups: Vec<ParsedReplayRollup>,
}

struct IndexSummary {
    receipt_refs: BTreeSet<String>,
    rollup_refs: BTreeSet<String>,
    first_divergence_refs: BTreeSet<String>,
    report_refs: BTreeSet<String>,
    final_state_refs: BTreeSet<String>,
    divergence_counts: BTreeMap<String, u64>,
    pass_count: u64,
    deny_count: u64,
    raw_receipt_count: u64,
    rollup_count: u64,
    total_count: u64,
}

fn collect_index_inputs(inputs: &[ReplayIndexInput]) -> Result<ParsedInputs> {
    let mut diagnostics = Vec::with_capacity(inputs.len());
    let mut receipts = Vec::with_capacity(inputs.len());
    let mut rollups = Vec::with_capacity(inputs.len());
    for input in inputs {
        let actual_ref = preserves_rail::canonical_hash(&input.value)?;
        if let Some(diagnostic) = expected_ref_diagnostic(input.expected_ref.as_deref(), &actual_ref)? {
            diagnostics.push(diagnostic);
            continue;
        }
        if let Ok(parsed) = parse_replay_verify_receipt(&input.value, &actual_ref) {
            receipts.push(parsed);
        } else if let Ok(parsed) = parse_replay_rollup_receipt(&input.value, &actual_ref) {
            rollups.push(parsed);
        } else {
            diagnostics.push(format!("replay index input {actual_ref} is neither verify receipt nor rollup"));
        }
    }
    Ok(ParsedInputs {
        diagnostics,
        receipts,
        rollups,
    })
}

fn expected_ref_diagnostic(expected_ref: Option<&str>, actual_ref: &str) -> Result<Option<String>> {
    let Some(expected_ref) = expected_ref else {
        return Ok(None);
    };
    preserves_rail::validate_content_ref(expected_ref)?;
    if expected_ref == actual_ref {
        Ok(None)
    } else {
        Ok(Some(format!("replay index ref mismatch expected={expected_ref} actual={actual_ref}")))
    }
}

fn summarize_index_inputs(receipts: &[ParsedReplayVerify], rollups: &[ParsedReplayRollup]) -> IndexSummary {
    let mut summary = empty_index_summary(receipts.len(), rollups.len());
    for parsed in receipts {
        summary.receipt_refs.insert(parsed.receipt_ref.clone());
        *summary.divergence_counts.entry(parsed.divergence.clone()).or_insert(0) += 1;
        if parsed.decision == "pass" {
            summary.pass_count += 1;
        } else {
            summary.deny_count += 1;
        }
        if let Some(reference) = &parsed.first_divergence_ref {
            summary.first_divergence_refs.insert(reference.clone());
        }
        summary.report_refs.extend(parsed.report_refs.iter().cloned());
        summary.final_state_refs.extend(parsed.final_state_refs.iter().cloned());
    }
    for parsed in rollups {
        summary.rollup_refs.insert(parsed.rollup_ref.clone());
        summary.receipt_refs.extend(parsed.receipt_refs.iter().cloned());
        summary.first_divergence_refs.extend(parsed.first_divergence_refs.iter().cloned());
        merge_divergence_counts(&mut summary.divergence_counts, &parsed.divergence_counts);
        summary.pass_count += parsed.pass_count;
        summary.deny_count += parsed.deny_count;
        summary.total_count += parsed.total_count;
    }
    summary
}

fn empty_index_summary(raw_count: usize, rollup_count: usize) -> IndexSummary {
    let raw_receipt_count = raw_count as u64;
    IndexSummary {
        receipt_refs: BTreeSet::new(),
        rollup_refs: BTreeSet::new(),
        first_divergence_refs: BTreeSet::new(),
        report_refs: BTreeSet::new(),
        final_state_refs: BTreeSet::new(),
        divergence_counts: BTreeMap::new(),
        pass_count: 0,
        deny_count: 0,
        raw_receipt_count,
        rollup_count: rollup_count as u64,
        total_count: raw_receipt_count,
    }
}

fn rollup_anomalies(rollups: &[ParsedReplayRollup]) -> Vec<String> {
    rollups
        .iter()
        .filter(|parsed| parsed.decision == "deny" && parsed.deny_count == 0)
        .map(|parsed| format!("replay rollup {} denied without denied receipt count", parsed.rollup_ref))
        .collect()
}

fn index_value(decision: &str, diagnostics: &[String], summary: &IndexSummary) -> IOValue {
    preserves_rail::record("deterministic-replay-index-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA),
        preserves_rail::record("decision", vec![preserves_rail::string(decision)]),
        preserves_rail::record("total-count", vec![preserves_rail::u64_value(summary.total_count)]),
        preserves_rail::record("pass-count", vec![preserves_rail::u64_value(summary.pass_count)]),
        preserves_rail::record("deny-count", vec![preserves_rail::u64_value(summary.deny_count)]),
        preserves_rail::record("raw-receipt-count", vec![preserves_rail::u64_value(summary.raw_receipt_count)]),
        preserves_rail::record("rollup-count", vec![preserves_rail::u64_value(summary.rollup_count)]),
        preserves_rail::record("receipt-refs", vec![refs_value(&summary.receipt_refs)]),
        preserves_rail::record("rollup-refs", vec![refs_value(&summary.rollup_refs)]),
        preserves_rail::record("divergence-counts", vec![divergence_counts_value(&summary.divergence_counts)]),
        preserves_rail::record("first-divergence-refs", vec![refs_value(&summary.first_divergence_refs)]),
        preserves_rail::record("report-refs", vec![refs_value(&summary.report_refs)]),
        preserves_rail::record("final-state-refs", vec![refs_value(&summary.final_state_refs)]),
        preserves_rail::record("diagnostics", vec![preserves_rail::sequence(
            diagnostics.iter().map(preserves_rail::string).collect(),
        )]),
        preserves_rail::sequence(index_checks(decision, diagnostics.is_empty())),
    ])
}

pub fn chaos_schedule_receipt(input: &ChaosScheduleInput) -> Result<ChaosScheduleReceipt> {
    preserves_rail::validate_content_ref(&input.seed_ref)?;
    preserves_rail::validate_content_ref(&input.event_ref)?;
    validate_chaos_fault_kind(&input.fault_kind)?;
    if input.intensity_percent > 100 {
        return Err(crate::error::MoltenError::invalid_harness("chaos schedule intensity exceeds 100"));
    }
    let preimage = preserves_rail::record("deterministic-chaos-schedule-preimage-v1", vec![
        preserves_rail::record("seed-ref", vec![preserves_rail::string(&input.seed_ref)]),
        preserves_rail::record("position", vec![preserves_rail::u64_value(input.schedule_position)]),
        preserves_rail::record("event-ref", vec![preserves_rail::string(&input.event_ref)]),
        preserves_rail::record("fault-kind", vec![preserves_rail::string(&input.fault_kind)]),
    ]);
    let sample_ref = preserves_rail::canonical_hash(&preimage)?;
    let sample = chaos_sample_percent(&sample_ref)?;
    let decision = if sample < input.intensity_percent {
        "inject"
    } else {
        "pass"
    };
    let value = preserves_rail::record("deterministic-chaos-schedule-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_CHAOS_SCHEDULE_SCHEMA),
        preserves_rail::record("seed-ref", vec![preserves_rail::string(&input.seed_ref)]),
        preserves_rail::record("position", vec![preserves_rail::u64_value(input.schedule_position)]),
        preserves_rail::record("event-ref", vec![preserves_rail::string(&input.event_ref)]),
        preserves_rail::record("fault-kind", vec![preserves_rail::string(&input.fault_kind)]),
        preserves_rail::record("intensity-percent", vec![preserves_rail::u64_value(input.intensity_percent)]),
        preserves_rail::record("sample-ref", vec![preserves_rail::string(&sample_ref)]),
        preserves_rail::record("decision", vec![preserves_rail::string(decision)]),
        preserves_rail::sequence(vec![
            preserves_rail::record("check", vec![
                preserves_rail::string("deterministic-schedule"),
                preserves_rail::string("pass"),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("replay-identity-bound"),
                preserves_rail::string("pass"),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("evidence-only-no-authority"),
                preserves_rail::string("pass"),
            ]),
        ]),
    ]);
    let schedule_ref = preserves_rail::canonical_hash(&value)?;
    Ok(ChaosScheduleReceipt {
        value,
        schedule_ref,
        decision: decision.to_string(),
    })
}

pub fn deterministic_integration_receipt(
    input: &DeterministicIntegrationInput,
) -> Result<DeterministicIntegrationReceipt> {
    validate_integration_kind(&input.integration_kind)?;
    preserves_rail::validate_content_ref(&input.handler_profile_ref)?;
    preserves_rail::validate_content_ref(&input.effect_log_ref)?;
    preserves_rail::validate_content_ref(&input.snapshot_ref)?;
    preserves_rail::validate_content_ref(&input.gate_ref)?;
    let decision = if input.admitted_live_effects { "deny" } else { "pass" };
    let value = preserves_rail::record("deterministic-integration-gate-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_INTEGRATION_GATE_SCHEMA),
        preserves_rail::record("integration-kind", vec![preserves_rail::string(&input.integration_kind)]),
        preserves_rail::record("decision", vec![preserves_rail::string(decision)]),
        preserves_rail::record("handler-profile-ref", vec![preserves_rail::string(&input.handler_profile_ref)]),
        preserves_rail::record("effect-log-ref", vec![preserves_rail::string(&input.effect_log_ref)]),
        preserves_rail::record("snapshot-ref", vec![preserves_rail::string(&input.snapshot_ref)]),
        preserves_rail::record("gate-ref", vec![preserves_rail::string(&input.gate_ref)]),
        preserves_rail::sequence(vec![
            preserves_rail::record("check", vec![
                preserves_rail::string("handler-profile-bound"),
                preserves_rail::string("pass"),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("effect-log-bound"),
                preserves_rail::string("pass"),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("snapshot-bound"),
                preserves_rail::string("pass"),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("no-live-effect-during-replay"),
                preserves_rail::string(if input.admitted_live_effects { "deny" } else { "pass" }),
            ]),
            preserves_rail::record("check", vec![
                preserves_rail::string("integration-gate-decision"),
                preserves_rail::string(decision),
            ]),
        ]),
    ]);
    let receipt_ref = preserves_rail::canonical_hash(&value)?;
    Ok(DeterministicIntegrationReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
    })
}

pub fn trace_privacy_receipt(input: &TracePrivacyInput) -> Result<TracePrivacyReceipt> {
    preserves_rail::validate_content_ref(&input.trace_ref)?;
    preserves_rail::validate_content_ref(&input.snapshot_ref)?;
    preserves_rail::validate_content_ref(&input.requester_ref)?;
    preserves_rail::validate_content_ref(&input.policy_ref)?;
    let decision = match (input.has_export_authority, input.contains_sensitive_refs) {
        (false, true) => "deny",
        (true, true) => "redacted",
        _ => "pass",
    };
    let value = preserves_rail::record("deterministic-trace-privacy-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_TRACE_PRIVACY_SCHEMA),
        preserves_rail::record("decision", vec![preserves_rail::string(decision)]),
        preserves_rail::record("trace-ref", vec![preserves_rail::string(&input.trace_ref)]),
        preserves_rail::record("snapshot-ref", vec![preserves_rail::string(&input.snapshot_ref)]),
        preserves_rail::record("requester-ref", vec![preserves_rail::string(&input.requester_ref)]),
        preserves_rail::record("policy-ref", vec![preserves_rail::string(&input.policy_ref)]),
        preserves_rail::record("contains-sensitive-refs", vec![preserves_rail::string(
            if input.contains_sensitive_refs { "yes" } else { "no" },
        )]),
        preserves_rail::sequence(trace_privacy_checks(
            decision,
            input.has_export_authority,
            input.contains_sensitive_refs,
        )),
    ]);
    let receipt_ref = preserves_rail::canonical_hash(&value)?;
    Ok(TracePrivacyReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
    })
}

fn trace_privacy_checks(decision: &str, has_export_authority: bool, contains_sensitive_refs: bool) -> Vec<IOValue> {
    vec![
        preserves_rail::record("check", vec![
            preserves_rail::string("policy-admission-before-render"),
            preserves_rail::string("pass"),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("sensitive-trace-gated"),
            preserves_rail::string(if !contains_sensitive_refs || has_export_authority {
                "pass"
            } else {
                "deny"
            }),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("redacted-view-when-authorized-sensitive"),
            preserves_rail::string(if decision == "redacted" || !contains_sensitive_refs {
                "pass"
            } else {
                "deny"
            }),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("trace-privacy-decision"),
            preserves_rail::string(decision),
        ]),
    ]
}

fn validate_integration_kind(kind: &str) -> Result<()> {
    match kind {
        "remote-sync" | "storage" | "job-dag" | "upgrade" => Ok(()),
        _ => Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported deterministic integration kind {kind}"
        ))),
    }
}

fn validate_chaos_fault_kind(kind: &str) -> Result<()> {
    match kind {
        "fault" | "delay" | "drop" | "reorder" | "partition" | "resource-limit" => Ok(()),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("unsupported chaos fault kind {kind}"))),
    }
}

fn chaos_sample_percent(sample_ref: &str) -> Result<u64> {
    let hex = preserves_rail::content_ref_hex(sample_ref)?;
    let sample = u64::from_str_radix(&hex[..16], 16)
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("invalid chaos sample ref: {error}")))?;
    Ok(sample % 100)
}

fn parse_replay_verify_receipt(value: &IOValue, receipt_ref: &str) -> Result<ParsedReplayVerify> {
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(13)) {
        require_schema_value(
            &fields[0],
            preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA,
            "deterministic replay verify",
        )?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[10], "divergence")?;
        let first_divergence_ref = record_string_value(&fields[11], "first-divergence-ref")?;
        validate_replay_decision(&decision)?;
        validate_divergence_ref(&first_divergence_ref)?;
        let expected_final_state_ref = record_string_value(&fields[8], "expected-final-state-ref")?;
        let actual_final_state_ref = record_string_value(&fields[9], "actual-final-state-ref")?;
        preserves_rail::validate_content_ref(&expected_final_state_ref)?;
        preserves_rail::validate_content_ref(&actual_final_state_ref)?;
        return Ok(ParsedReplayVerify {
            receipt_ref: receipt_ref.to_string(),
            decision,
            divergence,
            first_divergence_ref: (first_divergence_ref != "none").then_some(first_divergence_ref),
            report_refs: Vec::new(),
            final_state_refs: vec![expected_final_state_ref, actual_final_state_ref],
        });
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(7)) {
        require_schema_value(
            &fields[0],
            preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA,
            "deterministic replay verify",
        )?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[5], "divergence")?;
        validate_replay_decision(&decision)?;
        let expected_report_ref = record_string_value(&fields[2], "expected-report-ref")?;
        let actual_report_ref = record_string_value(&fields[3], "actual-report-ref")?;
        let final_state_ref = record_string_value(&fields[4], "final-state-ref")?;
        preserves_rail::validate_content_ref(&expected_report_ref)?;
        preserves_rail::validate_content_ref(&actual_report_ref)?;
        preserves_rail::validate_content_ref(&final_state_ref)?;
        return Ok(ParsedReplayVerify {
            receipt_ref: receipt_ref.to_string(),
            decision,
            divergence,
            first_divergence_ref: None,
            report_refs: vec![expected_report_ref, actual_report_ref],
            final_state_refs: vec![final_state_ref],
        });
    }
    Err(crate::error::MoltenError::invalid_harness("expected <deterministic-replay-verify-v1 ...>"))
}

fn parse_replay_rollup_receipt(value: &IOValue, rollup_ref: &str) -> Result<ParsedReplayRollup> {
    let fields = value
        .collect_simple_record("deterministic-replay-rollup-v1", Some(10))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-replay-rollup-v1 ...>"))?;
    require_schema_value(
        &fields[0],
        preserves_rail::DETERMINISTIC_REPLAY_ROLLUP_SCHEMA,
        "deterministic replay rollup",
    )?;
    let decision = record_string_value(&fields[1], "decision")?;
    validate_replay_decision(&decision)?;
    let total_count = record_u64_value(&fields[2], "total-count")?;
    let pass_count = record_u64_value(&fields[3], "pass-count")?;
    let deny_count = record_u64_value(&fields[4], "deny-count")?;
    let receipt_refs = record_ref_list_value(&fields[5], "receipt-refs")?;
    let divergence_counts = record_divergence_counts_value(&fields[6])?;
    let first_divergence_refs = record_ref_list_value(&fields[7], "first-divergence-refs")?;
    Ok(ParsedReplayRollup {
        rollup_ref: rollup_ref.to_string(),
        decision,
        total_count,
        pass_count,
        deny_count,
        receipt_refs,
        divergence_counts,
        first_divergence_refs,
    })
}

struct RunChoices {
    scenario_label: &'static str,
    policy_ref: &'static str,
    scheduler_key: &'static str,
    input_message: &'static str,
    request_payload: &'static str,
    response_payload: &'static str,
    decision: &'static str,
    action: &'static str,
    receipt: &'static str,
    output: &'static str,
    after_state: &'static str,
}

struct EffectRefs {
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
}

struct OutputRefs {
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
}

struct StateRefs {
    before_state_ref: String,
    after_state_ref: String,
}

fn run_parts(variant: ReplayFixtureVariant) -> Result<ReplayRunParts> {
    let choices = run_choices(variant);
    let identity = run_identity_value(choices.scenario_label, choices.policy_ref);
    let identity_ref = preserves_rail::canonical_hash(&identity)?;
    let effects = effect_refs(&choices, &identity_ref)?;
    let outputs = output_refs(&choices, &effects)?;
    let states = state_refs(&choices, &identity_ref, &outputs)?;
    let effect_log = effect_log_value(&effects);
    let effect_log_ref = preserves_rail::canonical_hash(&effect_log)?;
    let turn_journal = turn_journal_value(&effects, &outputs, &states);
    let turn_journal_ref = preserves_rail::canonical_hash(&turn_journal)?;
    Ok(ReplayRunParts {
        identity,
        identity_ref,
        scheduler_ref: effects.scheduler_ref,
        input_ref: effects.input_ref,
        effect_request_ref: effects.effect_request_ref,
        effect_response_ref: effects.effect_response_ref,
        policy_decision_ref: effects.policy_decision_ref,
        action_ref: outputs.action_ref,
        receipt_ref: outputs.receipt_ref,
        output_ref: outputs.output_ref,
        after_state_ref: states.after_state_ref,
        turn_journal,
        turn_journal_ref,
        effect_log,
        effect_log_ref,
    })
}

fn run_choices(variant: ReplayFixtureVariant) -> RunChoices {
    RunChoices {
        scenario_label: match variant {
            ReplayFixtureVariant::ChangedIdentity => "fixture:changed-identity",
            _ => "fixture:baseline",
        },
        policy_ref: match variant {
            ReplayFixtureVariant::ChangedIdentity => DEFAULT_REVOCATION_REF,
            _ => DEFAULT_POLICY_REF,
        },
        scheduler_key: match variant {
            ReplayFixtureVariant::ChangedScheduler => "logical:0:priority:1:queue:0:actor:helper",
            _ => "logical:0:priority:0:queue:0:actor:helper",
        },
        input_message: match variant {
            ReplayFixtureVariant::ChangedInput => "message:changed",
            _ => "message:root-to-helper",
        },
        request_payload: match variant {
            ReplayFixtureVariant::ChangedEffectRequest => "logical-now:changed-sequence",
            ReplayFixtureVariant::MissingRecordedEffect => "network:live-fetch",
            _ => "logical-now:turn-0001",
        },
        response_payload: match variant {
            ReplayFixtureVariant::ChangedEffectResponse => "logical-time:43",
            ReplayFixtureVariant::MissingRecordedEffect => "denied:missing-recorded-response",
            _ => "logical-time:42",
        },
        decision: match variant {
            ReplayFixtureVariant::ChangedPolicyDecision => "deny",
            _ => "pass",
        },
        action: match variant {
            ReplayFixtureVariant::ChangedAction => "assert:alternate-output",
            _ => "assert:helper-output",
        },
        receipt: match variant {
            ReplayFixtureVariant::ChangedReceipt => "receipt:alternate",
            _ => "receipt:turn-0001",
        },
        output: match variant {
            ReplayFixtureVariant::ChangedOutput => "output:alternate",
            _ => "output:helper-ack",
        },
        after_state: match variant {
            ReplayFixtureVariant::ChangedStateHash => "after:changed",
            _ => "after:committed",
        },
    }
}

fn effect_refs(choices: &RunChoices, identity_ref: &str) -> Result<EffectRefs> {
    let scheduler_ref =
        preserves_rail::canonical_hash(&preserves_rail::record("deterministic-scheduler-key-v1", vec![
            preserves_rail::string(choices.scheduler_key),
        ]))?;
    let input_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-fixture-input-v1", vec![
        preserves_rail::string(choices.input_message),
        preserves_rail::record("identity-ref", vec![preserves_rail::string(identity_ref)]),
    ]))?;
    let effect_request_ref =
        preserves_rail::canonical_hash(&preserves_rail::record("deterministic-effect-request-v1", vec![
            preserves_rail::string("clock"),
            preserves_rail::string(choices.request_payload),
            preserves_rail::record("input-ref", vec![preserves_rail::string(&input_ref)]),
            preserves_rail::record("profile", vec![preserves_rail::string("replay")]),
        ]))?;
    let effect_response_ref =
        preserves_rail::canonical_hash(&preserves_rail::record("deterministic-effect-response-v1", vec![
            preserves_rail::string(choices.response_payload),
            preserves_rail::record("request-ref", vec![preserves_rail::string(&effect_request_ref)]),
            preserves_rail::record("source", vec![preserves_rail::string("recorded-effect-log")]),
        ]))?;
    let policy_decision_ref =
        preserves_rail::canonical_hash(&preserves_rail::record("deterministic-policy-decision-v1", vec![
            preserves_rail::string(choices.decision),
            preserves_rail::record("policy-ref", vec![preserves_rail::string(choices.policy_ref)]),
            preserves_rail::record("input-ref", vec![preserves_rail::string(&input_ref)]),
            preserves_rail::record("effect-response-ref", vec![preserves_rail::string(&effect_response_ref)]),
        ]))?;
    Ok(EffectRefs {
        scheduler_ref,
        input_ref,
        effect_request_ref,
        effect_response_ref,
        policy_decision_ref,
    })
}

fn output_refs(choices: &RunChoices, effects: &EffectRefs) -> Result<OutputRefs> {
    let action_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-action-v1", vec![
        preserves_rail::string(choices.action),
        preserves_rail::record("policy-decision-ref", vec![preserves_rail::string(&effects.policy_decision_ref)]),
    ]))?;
    let receipt_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-turn-receipt-v1", vec![
        preserves_rail::string(choices.receipt),
        preserves_rail::record("action-ref", vec![preserves_rail::string(&action_ref)]),
    ]))?;
    let output_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-output-v1", vec![
        preserves_rail::string(choices.output),
        preserves_rail::record("receipt-ref", vec![preserves_rail::string(&receipt_ref)]),
    ]))?;
    Ok(OutputRefs {
        action_ref,
        receipt_ref,
        output_ref,
    })
}

fn state_refs(choices: &RunChoices, identity_ref: &str, outputs: &OutputRefs) -> Result<StateRefs> {
    let before_state_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-state-v1", vec![
        preserves_rail::string("before"),
        preserves_rail::record("identity-ref", vec![preserves_rail::string(identity_ref)]),
    ]))?;
    let after_state_ref = preserves_rail::canonical_hash(&preserves_rail::record("deterministic-state-v1", vec![
        preserves_rail::string(choices.after_state),
        preserves_rail::record("before-state-ref", vec![preserves_rail::string(&before_state_ref)]),
        preserves_rail::record("output-ref", vec![preserves_rail::string(&outputs.output_ref)]),
    ]))?;
    Ok(StateRefs {
        before_state_ref,
        after_state_ref,
    })
}

fn effect_log_value(effects: &EffectRefs) -> IOValue {
    preserves_rail::record("deterministic-effect-log-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_EFFECT_LOG_SCHEMA),
        preserves_rail::record("handler-profile-ref", vec![preserves_rail::string(DEFAULT_HANDLER_PROFILE_REF)]),
        preserves_rail::sequence(vec![preserves_rail::record("effect-entry-v1", vec![
            preserves_rail::record("sequence", vec![preserves_rail::string("0")]),
            preserves_rail::record("effect-kind", vec![preserves_rail::string("clock")]),
            preserves_rail::record("request-ref", vec![preserves_rail::string(&effects.effect_request_ref)]),
            preserves_rail::record("response-ref", vec![preserves_rail::string(&effects.effect_response_ref)]),
        ])]),
    ])
}

fn turn_journal_value(effects: &EffectRefs, outputs: &OutputRefs, states: &StateRefs) -> IOValue {
    preserves_rail::record("deterministic-turn-journal-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_TURN_JOURNAL_SCHEMA),
        preserves_rail::record("turn-id", vec![preserves_rail::string("turn:0001")]),
        preserves_rail::record("actor-id", vec![preserves_rail::string("actor:helper")]),
        preserves_rail::record("scheduler-key-ref", vec![preserves_rail::string(&effects.scheduler_ref)]),
        preserves_rail::record("input-ref", vec![preserves_rail::string(&effects.input_ref)]),
        preserves_rail::record("before-state-ref", vec![preserves_rail::string(&states.before_state_ref)]),
        preserves_rail::record("effect-request-ref", vec![preserves_rail::string(&effects.effect_request_ref)]),
        preserves_rail::record("effect-response-ref", vec![preserves_rail::string(&effects.effect_response_ref)]),
        preserves_rail::record("policy-decision-ref", vec![preserves_rail::string(&effects.policy_decision_ref)]),
        preserves_rail::record("action-ref", vec![preserves_rail::string(&outputs.action_ref)]),
        preserves_rail::record("receipt-ref", vec![preserves_rail::string(&outputs.receipt_ref)]),
        preserves_rail::record("output-ref", vec![preserves_rail::string(&outputs.output_ref)]),
        preserves_rail::record("after-state-ref", vec![preserves_rail::string(&states.after_state_ref)]),
    ])
}

fn run_identity_value(scenario_label: &'static str, policy_ref: &'static str) -> IOValue {
    preserves_rail::record("deterministic-run-identity-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_RUN_IDENTITY_SCHEMA),
        preserves_rail::record("scenario", vec![preserves_rail::string(scenario_label)]),
        preserves_rail::record("artifact-ref", vec![preserves_rail::string(DEFAULT_ARTIFACT_REF)]),
        preserves_rail::record("dependency-closure-ref", vec![preserves_rail::string(DEFAULT_CLOSURE_REF)]),
        preserves_rail::record("initial-state-ref", vec![preserves_rail::string(DEFAULT_INITIAL_STATE_REF)]),
        preserves_rail::sequence(vec![preserves_rail::string(DEFAULT_SCHEMA_REF)]),
        preserves_rail::sequence(vec![preserves_rail::string(policy_ref)]),
        preserves_rail::sequence(vec![preserves_rail::string(DEFAULT_CAPABILITY_REF)]),
        preserves_rail::sequence(vec![preserves_rail::string(DEFAULT_REVOCATION_REF)]),
        preserves_rail::record("handler-profile-ref", vec![preserves_rail::string(DEFAULT_HANDLER_PROFILE_REF)]),
        preserves_rail::record("seed-ref", vec![preserves_rail::string(DEFAULT_SEED_REF)]),
        preserves_rail::sequence(vec![
            preserves_rail::string(DEFAULT_RUNTIME_REF),
            preserves_rail::string(DEFAULT_TOOL_REF),
        ]),
    ])
}

fn first_divergence(
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
    variant: ReplayFixtureVariant,
) -> ReplayDivergenceKind {
    if variant == ReplayFixtureVariant::MissingRecordedEffect {
        return ReplayDivergenceKind::LiveEffect;
    }
    if expected.identity_ref != actual.identity_ref {
        return ReplayDivergenceKind::Identity;
    }
    if expected.scheduler_ref != actual.scheduler_ref {
        return ReplayDivergenceKind::Scheduler;
    }
    if expected.input_ref != actual.input_ref {
        return ReplayDivergenceKind::Input;
    }
    if expected.effect_request_ref != actual.effect_request_ref {
        return ReplayDivergenceKind::EffectRequest;
    }
    if expected.effect_response_ref != actual.effect_response_ref {
        return ReplayDivergenceKind::EffectResponse;
    }
    if expected.policy_decision_ref != actual.policy_decision_ref {
        return ReplayDivergenceKind::PolicyDecision;
    }
    if expected.action_ref != actual.action_ref {
        return ReplayDivergenceKind::Action;
    }
    if expected.receipt_ref != actual.receipt_ref {
        return ReplayDivergenceKind::Receipt;
    }
    if expected.output_ref != actual.output_ref {
        return ReplayDivergenceKind::Output;
    }
    if expected.after_state_ref != actual.after_state_ref {
        return ReplayDivergenceKind::StateHash;
    }
    ReplayDivergenceKind::None
}

fn first_divergence_value(
    kind: ReplayDivergenceKind,
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
) -> Result<IOValue> {
    let (expected_ref, actual_ref) = divergence_refs(kind, expected, actual);
    Ok(preserves_rail::record("deterministic-first-divergence-v1", vec![
        preserves_rail::string(preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
        preserves_rail::record("kind", vec![preserves_rail::string(kind.as_str())]),
        preserves_rail::record("turn-id", vec![preserves_rail::string("turn:0001")]),
        preserves_rail::record("actor-id", vec![preserves_rail::string("actor:helper")]),
        preserves_rail::record("log-position", vec![preserves_rail::string("0")]),
        preserves_rail::record("handler-profile-ref", vec![preserves_rail::string(DEFAULT_HANDLER_PROFILE_REF)]),
        preserves_rail::record("expected-ref", vec![preserves_rail::string(expected_ref)]),
        preserves_rail::record("actual-ref", vec![preserves_rail::string(actual_ref)]),
        preserves_rail::sequence(vec![
            preserves_rail::string("safe-canonical-refs-only"),
            preserves_rail::string("redact-secret-capability-material"),
        ]),
    ]))
}

fn divergence_refs<'a>(
    kind: ReplayDivergenceKind,
    expected: &'a ReplayRunParts,
    actual: &'a ReplayRunParts,
) -> (&'a str, &'a str) {
    match kind {
        ReplayDivergenceKind::Identity => (&expected.identity_ref, &actual.identity_ref),
        ReplayDivergenceKind::Scheduler => (&expected.scheduler_ref, &actual.scheduler_ref),
        ReplayDivergenceKind::Input => (&expected.input_ref, &actual.input_ref),
        ReplayDivergenceKind::EffectRequest | ReplayDivergenceKind::LiveEffect => {
            (&expected.effect_request_ref, &actual.effect_request_ref)
        }
        ReplayDivergenceKind::EffectResponse => (&expected.effect_response_ref, &actual.effect_response_ref),
        ReplayDivergenceKind::PolicyDecision => (&expected.policy_decision_ref, &actual.policy_decision_ref),
        ReplayDivergenceKind::Action => (&expected.action_ref, &actual.action_ref),
        ReplayDivergenceKind::Receipt => (&expected.receipt_ref, &actual.receipt_ref),
        ReplayDivergenceKind::Output => (&expected.output_ref, &actual.output_ref),
        ReplayDivergenceKind::StateHash => (&expected.after_state_ref, &actual.after_state_ref),
        ReplayDivergenceKind::None => (&expected.turn_journal_ref, &actual.turn_journal_ref),
    }
}

fn verify_checks(decision: &str, divergence: ReplayDivergenceKind) -> Vec<IOValue> {
    let replay_status = if decision == "pass" { "pass" } else { "deny" };
    vec![
        preserves_rail::record("check", vec![preserves_rail::string("identity-bound"), preserves_rail::string("pass")]),
        preserves_rail::record("check", vec![
            preserves_rail::string("ordered-boundary-comparison"),
            preserves_rail::string(replay_status),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("recorded-effects-only"),
            preserves_rail::string(replay_status),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("first-divergence"),
            preserves_rail::string(divergence.as_str()),
        ]),
    ]
}

fn rollup_checks(decision: &str, all_inputs_readable: bool) -> Vec<IOValue> {
    vec![
        preserves_rail::record("check", vec![preserves_rail::string("evidence-only"), preserves_rail::string("pass")]),
        preserves_rail::record("check", vec![
            preserves_rail::string("no-authority-grant"),
            preserves_rail::string("pass"),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("individual-receipts-required"),
            preserves_rail::string("pass"),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("all-inputs-readable"),
            preserves_rail::string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("rollup-decision"),
            preserves_rail::string(decision),
        ]),
    ]
}

fn index_checks(decision: &str, all_inputs_readable: bool) -> Vec<IOValue> {
    vec![
        preserves_rail::record("check", vec![preserves_rail::string("evidence-only"), preserves_rail::string("pass")]),
        preserves_rail::record("check", vec![
            preserves_rail::string("no-authority-grant"),
            preserves_rail::string("pass"),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("rollup-and-receipt-refs-verified"),
            preserves_rail::string("pass"),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("all-inputs-readable"),
            preserves_rail::string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        preserves_rail::record("check", vec![
            preserves_rail::string("index-decision"),
            preserves_rail::string(decision),
        ]),
    ]
}

fn refs_value(refs: &BTreeSet<String>) -> IOValue {
    preserves_rail::sequence(refs.iter().map(preserves_rail::string).collect())
}

fn divergence_counts_value(counts: &BTreeMap<String, u64>) -> IOValue {
    preserves_rail::sequence(
        counts
            .iter()
            .map(|(kind, count)| {
                preserves_rail::record("divergence-count", vec![
                    preserves_rail::string(kind),
                    preserves_rail::u64_value(*count),
                ])
            })
            .collect(),
    )
}

fn require_schema_value(value: &Value<IOValue>, schema: &str, label: &str) -> Result<()> {
    let actual = required_string_value(value, label)?;
    if actual == schema {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} schema mismatch: expected {schema}, got {actual}"
        )))
    }
}

fn record_string_value(value: &Value<IOValue>, label: &'static str) -> Result<String> {
    let value = preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string_value(&fields[0], label)
}

fn record_u64_value(value: &Value<IOValue>, label: &'static str) -> Result<u64> {
    let value = preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a u64")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("{label} out of range: {error}")))
}

fn record_ref_list_value(value: &Value<IOValue>, label: &'static str) -> Result<Vec<String>> {
    let value = preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = required_string_value(item, label)?;
        preserves_rail::validate_content_ref(&reference)?;
        refs.push(reference);
    }
    Ok(refs)
}

fn record_divergence_counts_value(value: &Value<IOValue>) -> Result<BTreeMap<String, u64>> {
    let value = preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("divergence-counts", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <divergence-counts ...>"))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("divergence-counts must be a sequence"))?;
    if items.len() > MAX_REPLAY_INDEX_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "divergence count entries exceed {MAX_REPLAY_INDEX_INPUTS}"
        )));
    }
    let mut count_entries = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = preserves_rail::value_to_iovalue(item);
        let count_fields = item
            .collect_simple_record("divergence-count", Some(2))
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <divergence-count ...>"))?;
        let kind = required_string_value(&count_fields[0], "divergence kind")?;
        let count = count_fields[1]
            .as_u64()
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("divergence count must be a u64"))?
            .map_err(|error| {
                crate::error::MoltenError::invalid_harness(format!("divergence count out of range: {error}"))
            })?;
        count_entries.push((kind, count));
    }
    Ok(count_entries.into_iter().collect())
}

fn merge_divergence_counts(target: &mut BTreeMap<String, u64>, source: &BTreeMap<String, u64>) {
    for (kind, count) in source {
        *target.entry(kind.clone()).or_insert(0) += count;
    }
}

fn required_string_value(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a string")))
}

fn validate_replay_decision(decision: &str) -> Result<()> {
    if decision == "pass" || decision == "deny" {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "replay decision must be pass or deny, got {decision}"
        )))
    }
}

fn validate_divergence_ref(reference: &str) -> Result<()> {
    if reference == "none" {
        Ok(())
    } else {
        preserves_rail::validate_content_ref(reference)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::ChaosScheduleInput;
    use super::DEFAULT_ARTIFACT_REF;
    use super::DEFAULT_CAPABILITY_REF;
    use super::DEFAULT_HANDLER_PROFILE_REF;
    use super::DEFAULT_INITIAL_STATE_REF;
    use super::DEFAULT_POLICY_REF;
    use super::DEFAULT_SEED_REF;
    use super::DeterministicIntegrationInput;
    use super::ReplayDivergenceKind;
    use super::ReplayFixtureVariant;
    use super::ReplayIndexInput;
    use super::ReplayRollupInput;
    use super::TracePrivacyInput;
    use super::chaos_schedule_receipt;
    use super::deterministic_integration_receipt;
    use super::index_replay_evidence;
    use super::record_fixture_value;
    use super::replay_snapshot_manifest_bundle;
    use super::rollup_replay_receipts;
    use super::trace_privacy_receipt;
    use super::verify_fixture_value;
    use crate::chunk_store;
    use crate::preserves_rail;
    use crate::runtime::PredicateDecision;
    use crate::runtime::RuntimeSnapshotAuthorityState;
    use crate::runtime::evaluate_snapshot_authority;

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove stale temp dir {path:?}: {error}"),
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn replay_fixture_record_binds_identity_effects_and_final_state() {
        let fixture = record_fixture_value().expect("fixture record");
        assert!(fixture.record_ref.starts_with("blake3:"));
        assert!(fixture.identity_ref.starts_with("blake3:"));
        assert!(fixture.effect_log_ref.starts_with("blake3:"));
        assert!(fixture.output_ref.starts_with("blake3:"));
        assert!(fixture.final_state_ref.starts_with("blake3:"));
        let text = preserves_rail::to_text(&fixture.value).expect("render fixture");
        assert!(text.contains("deterministic-fixture-record-v1"));
        assert!(text.contains("deterministic-run-identity-v1"));
        assert!(text.contains("artifact-ref"));
        assert!(text.contains("dependency-closure-ref"));
        assert!(text.contains("initial-state-ref"));
        assert!(text.contains("handler-profile-ref"));
        assert!(text.contains("seed-ref"));
        assert!(text.contains("deterministic-effect-log-v1"));
        assert!(text.contains("effect-entry-v1"));
        assert!(text.contains("request-ref"));
        assert!(text.contains("response-ref"));
        assert!(text.contains("no-ambient-observations"));
    }

    #[test]
    fn unchanged_replay_passes_and_binds_output_refs() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("verify baseline");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::None);
        assert!(receipt.first_divergence.is_none());
        assert_eq!(receipt.receipt_ref, preserves_rail::canonical_hash(&receipt.value).expect("receipt hash"));
    }

    #[test]
    fn replay_reports_first_divergence_matrix() {
        let cases = [
            (ReplayFixtureVariant::ChangedIdentity, ReplayDivergenceKind::Identity),
            (ReplayFixtureVariant::ChangedScheduler, ReplayDivergenceKind::Scheduler),
            (ReplayFixtureVariant::ChangedInput, ReplayDivergenceKind::Input),
            (ReplayFixtureVariant::ChangedEffectRequest, ReplayDivergenceKind::EffectRequest),
            (ReplayFixtureVariant::ChangedEffectResponse, ReplayDivergenceKind::EffectResponse),
            (ReplayFixtureVariant::ChangedPolicyDecision, ReplayDivergenceKind::PolicyDecision),
            (ReplayFixtureVariant::ChangedAction, ReplayDivergenceKind::Action),
            (ReplayFixtureVariant::ChangedReceipt, ReplayDivergenceKind::Receipt),
            (ReplayFixtureVariant::ChangedOutput, ReplayDivergenceKind::Output),
            (ReplayFixtureVariant::ChangedStateHash, ReplayDivergenceKind::StateHash),
        ];
        for (variant, expected) in cases {
            let receipt = verify_fixture_value(variant).expect("verify divergent fixture");
            assert_eq!(receipt.decision, "deny");
            assert_eq!(receipt.divergence, expected);
            let divergence = receipt.first_divergence.expect("first divergence");
            let text = preserves_rail::to_text(&divergence).expect("render divergence");
            assert!(text.contains(expected.as_str()));
            assert!(text.contains("safe-canonical-refs-only"));
        }
    }

    #[test]
    fn replay_profile_denies_live_external_effects() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::MissingRecordedEffect).expect("verify missing effect");
        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::LiveEffect);
        let text = preserves_rail::to_text(&receipt.value).expect("render receipt");
        assert!(text.contains("recorded-effects-only"));
        assert!(text.contains("live-effect"));
    }

    #[test]
    fn replay_rollup_summarizes_pass_deny_and_divergence_counts() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let deny = verify_fixture_value(ReplayFixtureVariant::ChangedEffectResponse).expect("deny replay");
        let rollup = rollup_replay_receipts(&[
            ReplayRollupInput {
                expected_ref: Some(pass.receipt_ref.clone()),
                value: pass.value,
            },
            ReplayRollupInput {
                expected_ref: Some(deny.receipt_ref.clone()),
                value: deny.value,
            },
        ])
        .expect("rollup replay receipts");
        assert_eq!(rollup.decision, "deny");
        assert_eq!(rollup.total_count, 2);
        assert_eq!(rollup.pass_count, 1);
        assert_eq!(rollup.deny_count, 1);
        assert_eq!(rollup.rollup_ref, preserves_rail::canonical_hash(&rollup.value).expect("rollup hash"));
        let text = preserves_rail::to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("deterministic-replay-rollup-v1"));
        assert!(text.contains("effect-response"));
        assert!(text.contains("individual-receipts-required"));
    }

    #[test]
    fn replay_rollup_denies_mismatched_receipt_refs_without_counting_them() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let wrong_ref =
            preserves_rail::canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(wrong_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        assert_eq!(rollup.decision, "deny");
        assert_eq!(rollup.total_count, 0);
        let text = preserves_rail::to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("replay receipt ref mismatch"));
        assert!(text.contains(&wrong_ref));
        assert!(text.contains("all-inputs-readable"));
    }

    #[test]
    fn replay_snapshots_and_logs_are_manifest_backed_for_partial_debug_fetch() {
        let root = temp_dir("replay-snapshot-manifests");
        let bundle = replay_snapshot_manifest_bundle(&root, ReplayFixtureVariant::ChangedEffectResponse)
            .expect("snapshot manifest bundle");
        assert!(bundle.bundle_ref.starts_with("blake3:"));
        assert!(bundle.effect_log_manifest_ref.starts_with("blake3:"));
        assert!(bundle.turn_journal_manifest_ref.starts_with("blake3:"));
        assert!(bundle.snapshot_manifest_ref.starts_with("blake3:"));
        let first_divergence_manifest_ref =
            bundle.first_divergence_manifest_ref.as_ref().expect("first divergence manifest ref");
        assert!(first_divergence_manifest_ref.starts_with("blake3:"));
        assert!(bundle.debug_range_receipt_ref.as_ref().expect("range receipt").starts_with("blake3:"));
        let effect_log_read =
            chunk_store::read_object(&root, &bundle.effect_log_manifest_ref).expect("read effect log");
        assert!(crate::preserves_rail::parse_canonical_bytes(&effect_log_read.bytes).is_ok());
        let range =
            chunk_store::range_read(&root, first_divergence_manifest_ref, 0, 16).expect("partial divergence range");
        assert_eq!(range.bytes.len(), 16);
        let text = crate::preserves_rail::to_text(&bundle.value).expect("render bundle");
        assert!(text.contains("partial-divergence-debug-fetch"));
    }

    #[test]
    fn replay_index_combines_rollups_and_raw_receipts() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let deny = verify_fixture_value(ReplayFixtureVariant::ChangedOutput).expect("deny replay");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(pass.receipt_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        let index = index_replay_evidence(&[
            ReplayIndexInput {
                expected_ref: Some(rollup.rollup_ref.clone()),
                value: rollup.value,
            },
            ReplayIndexInput {
                expected_ref: Some(deny.receipt_ref.clone()),
                value: deny.value,
            },
        ])
        .expect("index replay evidence");
        assert_eq!(index.decision, "deny");
        assert_eq!(index.total_count, 2);
        assert_eq!(index.pass_count, 1);
        assert_eq!(index.deny_count, 1);
        assert_eq!(index.raw_receipt_count, 1);
        assert_eq!(index.rollup_count, 1);
        assert_eq!(index.index_ref, preserves_rail::canonical_hash(&index.value).expect("index hash"));
        let text = preserves_rail::to_text(&index.value).expect("render index");
        assert!(text.contains("deterministic-replay-index-v1"));
        assert!(text.contains("rollup-and-receipt-refs-verified"));
        assert!(text.contains("output"));
    }

    #[test]
    fn replay_index_denies_mismatched_rollup_ref() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(pass.receipt_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        let wrong_ref =
            preserves_rail::canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let index = index_replay_evidence(&[ReplayIndexInput {
            expected_ref: Some(wrong_ref.clone()),
            value: rollup.value,
        }])
        .expect("index replay evidence");
        assert_eq!(index.decision, "deny");
        assert_eq!(index.total_count, 0);
        let text = preserves_rail::to_text(&index.value).expect("render index");
        assert!(text.contains("replay index ref mismatch"));
        assert!(text.contains(&wrong_ref));
    }

    #[test]
    fn deterministic_integration_gates_bind_recorded_replay_inputs() {
        for integration_kind in ["remote-sync", "storage", "job-dag", "upgrade"] {
            let receipt = deterministic_integration_receipt(&DeterministicIntegrationInput {
                integration_kind: integration_kind.to_string(),
                handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
                effect_log_ref: DEFAULT_SEED_REF.to_string(),
                snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
                gate_ref: DEFAULT_ARTIFACT_REF.to_string(),
                admitted_live_effects: false,
            })
            .expect("integration receipt");
            assert_eq!(receipt.decision, "pass");
            assert_eq!(receipt.receipt_ref, preserves_rail::canonical_hash(&receipt.value).expect("receipt ref"));
            let text = preserves_rail::to_text(&receipt.value).expect("render integration receipt");
            assert!(text.contains(integration_kind));
            assert!(text.contains("handler-profile-bound"));
            assert!(text.contains("effect-log-bound"));
            assert!(text.contains("snapshot-bound"));
        }
        let denied = deterministic_integration_receipt(&DeterministicIntegrationInput {
            integration_kind: "remote-sync".to_string(),
            handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
            effect_log_ref: DEFAULT_SEED_REF.to_string(),
            snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            gate_ref: DEFAULT_ARTIFACT_REF.to_string(),
            admitted_live_effects: true,
        })
        .expect("integration denial");
        assert_eq!(denied.decision, "deny");
        assert!(
            preserves_rail::to_text(&denied.value)
                .expect("denial text")
                .contains("no-live-effect-during-replay")
        );
    }

    #[test]
    fn trace_privacy_gates_sensitive_trace_and_snapshot_exports() {
        let input = TracePrivacyInput {
            trace_ref: DEFAULT_ARTIFACT_REF.to_string(),
            snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            requester_ref: DEFAULT_CAPABILITY_REF.to_string(),
            policy_ref: DEFAULT_POLICY_REF.to_string(),
            has_export_authority: false,
            contains_sensitive_refs: true,
        };
        let denied = trace_privacy_receipt(&input).expect("trace privacy deny");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.receipt_ref, preserves_rail::canonical_hash(&denied.value).expect("privacy receipt ref"));
        let denied_text = preserves_rail::to_text(&denied.value).expect("render denied privacy receipt");
        assert!(denied_text.contains("policy-admission-before-render"));
        assert!(denied_text.contains("sensitive-trace-gated"));

        let redacted = trace_privacy_receipt(&TracePrivacyInput {
            has_export_authority: true,
            ..input.clone()
        })
        .expect("trace privacy redacted");
        assert_eq!(redacted.decision, "redacted");
        let redacted_text = preserves_rail::to_text(&redacted.value).expect("render redacted privacy receipt");
        assert!(redacted_text.contains("redacted-view-when-authorized-sensitive"));

        let public = trace_privacy_receipt(&TracePrivacyInput {
            contains_sensitive_refs: false,
            ..input
        })
        .expect("trace privacy public");
        assert_eq!(public.decision, "pass");
    }

    #[test]
    fn chaos_schedule_is_deterministic_replay_evidence_only() {
        let input = ChaosScheduleInput {
            seed_ref: DEFAULT_SEED_REF.to_string(),
            schedule_position: 7,
            event_ref: DEFAULT_ARTIFACT_REF.to_string(),
            fault_kind: "drop".to_string(),
            intensity_percent: 50,
        };
        let first = chaos_schedule_receipt(&input).expect("chaos schedule");
        let second = chaos_schedule_receipt(&input).expect("chaos schedule repeat");
        assert_eq!(first.schedule_ref, second.schedule_ref);
        assert_eq!(first.decision, second.decision);
        let text = preserves_rail::to_text(&first.value).expect("render chaos schedule");
        assert!(text.contains("deterministic-chaos-schedule-v1"));
        assert!(text.contains("replay-identity-bound"));
        assert!(text.contains("evidence-only-no-authority"));

        let changed = chaos_schedule_receipt(&ChaosScheduleInput {
            schedule_position: 8,
            ..input
        })
        .expect("changed chaos schedule");
        assert_ne!(first.schedule_ref, changed.schedule_ref);
        assert!(
            chaos_schedule_receipt(&ChaosScheduleInput {
                seed_ref: DEFAULT_SEED_REF.to_string(),
                schedule_position: 7,
                event_ref: DEFAULT_ARTIFACT_REF.to_string(),
                fault_kind: "drop".to_string(),
                intensity_percent: 101,
            })
            .is_err()
        );
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_replay_identity_scheduler_trace_and_snapshot_properties(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(10_000));
        let first = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("first baseline replay");
        let second = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("second baseline replay");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.decision, "pass");
        assert_eq!(first.divergence, ReplayDivergenceKind::None);
        let first_text = preserves_rail::to_text(&first.value).expect("render first replay");
        assert!(first_text.contains("ordered-boundary-comparison"));
        assert!(first_text.contains("recorded-effects-only"));

        let trace_a = record_fixture_value().expect("first fixture record");
        let trace_b = record_fixture_value().expect("second fixture record");
        assert_eq!(trace_a.record_ref, trace_b.record_ref);
        assert_eq!(trace_a.effect_log_ref, trace_b.effect_log_ref);
        assert_eq!(trace_a.final_state_ref, trace_b.final_state_ref);
        let trace_text = preserves_rail::to_text(&trace_a.value).expect("render fixture record");
        assert!(trace_text.contains("no-ambient-observations"));

        let variant = if salt.is_multiple_of(2) {
            ReplayFixtureVariant::ChangedScheduler
        } else {
            ReplayFixtureVariant::Baseline
        };
        let scheduler_check = verify_fixture_value(variant).expect("scheduler replay check");
        if variant == ReplayFixtureVariant::ChangedScheduler {
            assert_eq!(scheduler_check.decision, "deny");
            assert_eq!(scheduler_check.divergence, ReplayDivergenceKind::Scheduler);
        } else {
            assert_eq!(scheduler_check.decision, "pass");
        }

        let snapshot_ref =
            preserves_rail::canonical_hash(&preserves_rail::string(format!("snapshot-{salt}"))).expect("snapshot ref");
        let admitted_ref =
            preserves_rail::canonical_hash(&preserves_rail::string(format!("admitted-{salt}"))).expect("admitted ref");
        let redacted_ref =
            preserves_rail::canonical_hash(&preserves_rail::string(format!("redacted-{salt}"))).expect("redacted ref");
        let mut requested_refs = vec![admitted_ref.clone(), redacted_ref.clone()];
        requested_refs.sort();
        let snapshot_state = RuntimeSnapshotAuthorityState {
            snapshot_ref,
            admitted_authority_refs: vec![admitted_ref.clone()],
            claimed_authority_refs: vec![admitted_ref.clone()],
            requested_assertion_refs: requested_refs,
            readable_assertion_refs: vec![admitted_ref],
            redacted_assertion_refs: vec![redacted_ref],
        };
        let snapshot = evaluate_snapshot_authority(&snapshot_state).expect("snapshot authority predicate");
        assert!(snapshot.is_allowed);
        assert_eq!(snapshot.receipt.decision, PredicateDecision::Pass);
    }
}
