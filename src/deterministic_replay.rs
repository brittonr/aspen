use std::collections::BTreeMap;
use std::collections::BTreeSet;

use preserves::IOValue;
use preserves::Value;

use crate::error::Result;
use crate::preserves_rail::DETERMINISTIC_EFFECT_LOG_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_FIXTURE_RECORD_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_REPLAY_ROLLUP_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_RUN_IDENTITY_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_TURN_JOURNAL_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

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
    let parts = replay_run_parts(ReplayFixtureVariant::Baseline)?;
    let value = record("deterministic-fixture-record-v1", vec![
        string(DETERMINISTIC_FIXTURE_RECORD_SCHEMA),
        record("identity-ref", vec![string(&parts.identity_ref)]),
        parts.identity,
        record("effect-log-ref", vec![string(&parts.effect_log_ref)]),
        parts.effect_log,
        sequence(vec![parts.turn_journal]),
        record("output-ref", vec![string(&parts.output_ref)]),
        record("final-state-ref", vec![string(&parts.after_state_ref)]),
        sequence(vec![
            string("recorded-responses-bound"),
            string("canonical-journal-order"),
            string("no-ambient-observations"),
        ]),
    ]);
    let record_ref = canonical_hash(&value)?;
    Ok(ReplayFixtureRecord {
        value,
        record_ref,
        identity_ref: parts.identity_ref,
        effect_log_ref: parts.effect_log_ref,
        final_state_ref: parts.after_state_ref,
        output_ref: parts.output_ref,
    })
}

pub fn verify_fixture_value(variant: ReplayFixtureVariant) -> Result<ReplayVerifyReceipt> {
    let expected = replay_run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = replay_run_parts(variant)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let first_divergence = if divergence == ReplayDivergenceKind::None {
        None
    } else {
        Some(first_divergence_value(divergence, &expected, &actual)?)
    };
    let first_divergence_ref = match &first_divergence {
        Some(value) => canonical_hash(value)?,
        None => "none".to_string(),
    };
    let decision = if divergence == ReplayDivergenceKind::None {
        "pass"
    } else {
        "deny"
    };
    let value = record("deterministic-replay-verify-v1", vec![
        string(DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        string(decision),
        record("expected-identity-ref", vec![string(&expected.identity_ref)]),
        record("actual-identity-ref", vec![string(&actual.identity_ref)]),
        record("expected-effect-log-ref", vec![string(&expected.effect_log_ref)]),
        record("actual-effect-log-ref", vec![string(&actual.effect_log_ref)]),
        record("expected-output-ref", vec![string(&expected.output_ref)]),
        record("actual-output-ref", vec![string(&actual.output_ref)]),
        record("expected-final-state-ref", vec![string(&expected.after_state_ref)]),
        record("actual-final-state-ref", vec![string(&actual.after_state_ref)]),
        record("divergence", vec![string(divergence.as_str())]),
        record("first-divergence-ref", vec![string(&first_divergence_ref)]),
        sequence(verify_checks(decision, divergence)),
    ]);
    let receipt_ref = canonical_hash(&value)?;
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
        let actual_ref = canonical_hash(&input.value)?;
        if let Some(expected_ref) = input.expected_ref.as_deref() {
            validate_content_ref(expected_ref)?;
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
    let value = record("deterministic-replay-rollup-v1", vec![
        string(DETERMINISTIC_REPLAY_ROLLUP_SCHEMA),
        record("decision", vec![string(decision)]),
        record("total-count", vec![u64_value(total_count)]),
        record("pass-count", vec![u64_value(pass_count)]),
        record("deny-count", vec![u64_value(deny_count)]),
        record("receipt-refs", vec![refs_value(&receipt_refs)]),
        record("divergence-counts", vec![divergence_counts_value(&divergence_counts)]),
        record("first-divergence-refs", vec![refs_value(&first_divergence_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        sequence(rollup_checks(decision, diagnostics.is_empty())),
    ]);
    let rollup_ref = canonical_hash(&value)?;
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
    let mut diagnostics = Vec::with_capacity(inputs.len());
    let mut parsed_receipts = Vec::with_capacity(inputs.len());
    let mut parsed_rollups = Vec::with_capacity(inputs.len());
    for input in inputs {
        let actual_ref = canonical_hash(&input.value)?;
        if let Some(expected_ref) = input.expected_ref.as_deref() {
            validate_content_ref(expected_ref)?;
            if expected_ref != actual_ref {
                diagnostics.push(format!("replay index ref mismatch expected={expected_ref} actual={actual_ref}"));
                continue;
            }
        }
        if let Ok(parsed) = parse_replay_verify_receipt(&input.value, &actual_ref) {
            parsed_receipts.push(parsed);
        } else if let Ok(parsed) = parse_replay_rollup_receipt(&input.value, &actual_ref) {
            parsed_rollups.push(parsed);
        } else {
            diagnostics.push(format!("replay index input {actual_ref} is neither verify receipt nor rollup"));
        }
    }

    let mut receipt_refs = BTreeSet::new();
    let mut rollup_refs = BTreeSet::new();
    let mut first_divergence_refs = BTreeSet::new();
    let mut report_refs = BTreeSet::new();
    let mut final_state_refs = BTreeSet::new();
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
        report_refs.extend(parsed.report_refs.iter().cloned());
        final_state_refs.extend(parsed.final_state_refs.iter().cloned());
    }
    for parsed in &parsed_rollups {
        rollup_refs.insert(parsed.rollup_ref.clone());
        receipt_refs.extend(parsed.receipt_refs.iter().cloned());
        first_divergence_refs.extend(parsed.first_divergence_refs.iter().cloned());
        merge_divergence_counts(&mut divergence_counts, &parsed.divergence_counts);
        pass_count += parsed.pass_count;
        deny_count += parsed.deny_count;
        if parsed.decision == "deny" && parsed.deny_count == 0 {
            diagnostics.push(format!("replay rollup {} denied without denied receipt count", parsed.rollup_ref));
        }
    }
    let raw_receipt_count = parsed_receipts.len() as u64;
    let rollup_count = parsed_rollups.len() as u64;
    let total_count = raw_receipt_count + parsed_rollups.iter().map(|rollup| rollup.total_count).sum::<u64>();
    let decision = if diagnostics.is_empty() && deny_count == 0 {
        "pass"
    } else {
        "deny"
    };
    let value = record("deterministic-replay-index-v1", vec![
        string(DETERMINISTIC_REPLAY_INDEX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("total-count", vec![u64_value(total_count)]),
        record("pass-count", vec![u64_value(pass_count)]),
        record("deny-count", vec![u64_value(deny_count)]),
        record("raw-receipt-count", vec![u64_value(raw_receipt_count)]),
        record("rollup-count", vec![u64_value(rollup_count)]),
        record("receipt-refs", vec![refs_value(&receipt_refs)]),
        record("rollup-refs", vec![refs_value(&rollup_refs)]),
        record("divergence-counts", vec![divergence_counts_value(&divergence_counts)]),
        record("first-divergence-refs", vec![refs_value(&first_divergence_refs)]),
        record("report-refs", vec![refs_value(&report_refs)]),
        record("final-state-refs", vec![refs_value(&final_state_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        sequence(index_checks(decision, diagnostics.is_empty())),
    ]);
    let index_ref = canonical_hash(&value)?;
    Ok(ReplayIndexReceipt {
        value,
        index_ref,
        decision: decision.to_string(),
        total_count,
        pass_count,
        deny_count,
        raw_receipt_count,
        rollup_count,
    })
}

fn parse_replay_verify_receipt(value: &IOValue, receipt_ref: &str) -> Result<ParsedReplayVerify> {
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(13)) {
        require_schema_value(&fields[0], DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "deterministic replay verify")?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[10], "divergence")?;
        let first_divergence_ref = record_string_value(&fields[11], "first-divergence-ref")?;
        validate_replay_decision(&decision)?;
        validate_divergence_ref(&first_divergence_ref)?;
        let expected_final_state_ref = record_string_value(&fields[8], "expected-final-state-ref")?;
        let actual_final_state_ref = record_string_value(&fields[9], "actual-final-state-ref")?;
        validate_content_ref(&expected_final_state_ref)?;
        validate_content_ref(&actual_final_state_ref)?;
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
        require_schema_value(&fields[0], DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "deterministic replay verify")?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[5], "divergence")?;
        validate_replay_decision(&decision)?;
        let expected_report_ref = record_string_value(&fields[2], "expected-report-ref")?;
        let actual_report_ref = record_string_value(&fields[3], "actual-report-ref")?;
        let final_state_ref = record_string_value(&fields[4], "final-state-ref")?;
        validate_content_ref(&expected_report_ref)?;
        validate_content_ref(&actual_report_ref)?;
        validate_content_ref(&final_state_ref)?;
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
    require_schema_value(&fields[0], DETERMINISTIC_REPLAY_ROLLUP_SCHEMA, "deterministic replay rollup")?;
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

fn replay_run_parts(variant: ReplayFixtureVariant) -> Result<ReplayRunParts> {
    let scenario_label = match variant {
        ReplayFixtureVariant::ChangedIdentity => "fixture:changed-identity",
        _ => "fixture:baseline",
    };
    let policy_ref = match variant {
        ReplayFixtureVariant::ChangedIdentity => DEFAULT_REVOCATION_REF,
        _ => DEFAULT_POLICY_REF,
    };
    let identity = run_identity_value(scenario_label, policy_ref);
    let identity_ref = canonical_hash(&identity)?;
    let scheduler_ref = canonical_hash(&record("deterministic-scheduler-key-v1", vec![string(match variant {
        ReplayFixtureVariant::ChangedScheduler => "logical:0:priority:1:queue:0:actor:helper",
        _ => "logical:0:priority:0:queue:0:actor:helper",
    })]))?;
    let input_ref = canonical_hash(&record("deterministic-fixture-input-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedInput => "message:changed",
            _ => "message:root-to-helper",
        }),
        record("identity-ref", vec![string(&identity_ref)]),
    ]))?;
    let effect_request_ref = canonical_hash(&record("deterministic-effect-request-v1", vec![
        string("clock"),
        string(match variant {
            ReplayFixtureVariant::ChangedEffectRequest => "logical-now:changed-sequence",
            ReplayFixtureVariant::MissingRecordedEffect => "network:live-fetch",
            _ => "logical-now:turn-0001",
        }),
        record("input-ref", vec![string(&input_ref)]),
        record("profile", vec![string("replay")]),
    ]))?;
    let effect_response_ref = canonical_hash(&record("deterministic-effect-response-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedEffectResponse => "logical-time:43",
            ReplayFixtureVariant::MissingRecordedEffect => "denied:missing-recorded-response",
            _ => "logical-time:42",
        }),
        record("request-ref", vec![string(&effect_request_ref)]),
        record("source", vec![string("recorded-effect-log")]),
    ]))?;
    let policy_decision_ref = canonical_hash(&record("deterministic-policy-decision-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedPolicyDecision => "deny",
            _ => "pass",
        }),
        record("policy-ref", vec![string(policy_ref)]),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
    ]))?;
    let action_ref = canonical_hash(&record("deterministic-action-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedAction => "assert:alternate-output",
            _ => "assert:helper-output",
        }),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
    ]))?;
    let receipt_ref = canonical_hash(&record("deterministic-turn-receipt-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedReceipt => "receipt:alternate",
            _ => "receipt:turn-0001",
        }),
        record("action-ref", vec![string(&action_ref)]),
    ]))?;
    let output_ref = canonical_hash(&record("deterministic-output-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedOutput => "output:alternate",
            _ => "output:helper-ack",
        }),
        record("receipt-ref", vec![string(&receipt_ref)]),
    ]))?;
    let before_state_ref = canonical_hash(&record("deterministic-state-v1", vec![
        string("before"),
        record("identity-ref", vec![string(&identity_ref)]),
    ]))?;
    let after_state_ref = canonical_hash(&record("deterministic-state-v1", vec![
        string(match variant {
            ReplayFixtureVariant::ChangedStateHash => "after:changed",
            _ => "after:committed",
        }),
        record("before-state-ref", vec![string(&before_state_ref)]),
        record("output-ref", vec![string(&output_ref)]),
    ]))?;
    let effect_log = record("deterministic-effect-log-v1", vec![
        string(DETERMINISTIC_EFFECT_LOG_SCHEMA),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        sequence(vec![record("effect-entry-v1", vec![
            record("sequence", vec![string("0")]),
            record("effect-kind", vec![string("clock")]),
            record("request-ref", vec![string(&effect_request_ref)]),
            record("response-ref", vec![string(&effect_response_ref)]),
        ])]),
    ]);
    let effect_log_ref = canonical_hash(&effect_log)?;
    let turn_journal = record("deterministic-turn-journal-v1", vec![
        string(DETERMINISTIC_TURN_JOURNAL_SCHEMA),
        record("turn-id", vec![string("turn:0001")]),
        record("actor-id", vec![string("actor:helper")]),
        record("scheduler-key-ref", vec![string(&scheduler_ref)]),
        record("input-ref", vec![string(&input_ref)]),
        record("before-state-ref", vec![string(&before_state_ref)]),
        record("effect-request-ref", vec![string(&effect_request_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("action-ref", vec![string(&action_ref)]),
        record("receipt-ref", vec![string(&receipt_ref)]),
        record("output-ref", vec![string(&output_ref)]),
        record("after-state-ref", vec![string(&after_state_ref)]),
    ]);
    let turn_journal_ref = canonical_hash(&turn_journal)?;
    Ok(ReplayRunParts {
        identity,
        identity_ref,
        scheduler_ref,
        input_ref,
        effect_request_ref,
        effect_response_ref,
        policy_decision_ref,
        action_ref,
        receipt_ref,
        output_ref,
        after_state_ref,
        turn_journal,
        turn_journal_ref,
        effect_log,
        effect_log_ref,
    })
}

fn run_identity_value(scenario_label: &'static str, policy_ref: &'static str) -> IOValue {
    record("deterministic-run-identity-v1", vec![
        string(DETERMINISTIC_RUN_IDENTITY_SCHEMA),
        record("scenario", vec![string(scenario_label)]),
        record("artifact-ref", vec![string(DEFAULT_ARTIFACT_REF)]),
        record("dependency-closure-ref", vec![string(DEFAULT_CLOSURE_REF)]),
        record("initial-state-ref", vec![string(DEFAULT_INITIAL_STATE_REF)]),
        sequence(vec![string(DEFAULT_SCHEMA_REF)]),
        sequence(vec![string(policy_ref)]),
        sequence(vec![string(DEFAULT_CAPABILITY_REF)]),
        sequence(vec![string(DEFAULT_REVOCATION_REF)]),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        record("seed-ref", vec![string(DEFAULT_SEED_REF)]),
        sequence(vec![string(DEFAULT_RUNTIME_REF), string(DEFAULT_TOOL_REF)]),
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
    Ok(record("deterministic-first-divergence-v1", vec![
        string(DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
        record("kind", vec![string(kind.as_str())]),
        record("turn-id", vec![string("turn:0001")]),
        record("actor-id", vec![string("actor:helper")]),
        record("log-position", vec![string("0")]),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        record("expected-ref", vec![string(expected_ref)]),
        record("actual-ref", vec![string(actual_ref)]),
        sequence(vec![
            string("safe-canonical-refs-only"),
            string("redact-secret-capability-material"),
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
        record("check", vec![string("identity-bound"), string("pass")]),
        record("check", vec![string("ordered-boundary-comparison"), string(replay_status)]),
        record("check", vec![string("recorded-effects-only"), string(replay_status)]),
        record("check", vec![string("first-divergence"), string(divergence.as_str())]),
    ]
}

fn rollup_checks(decision: &str, all_inputs_readable: bool) -> Vec<IOValue> {
    vec![
        record("check", vec![string("evidence-only"), string("pass")]),
        record("check", vec![string("no-authority-grant"), string("pass")]),
        record("check", vec![string("individual-receipts-required"), string("pass")]),
        record("check", vec![
            string("all-inputs-readable"),
            string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        record("check", vec![string("rollup-decision"), string(decision)]),
    ]
}

fn index_checks(decision: &str, all_inputs_readable: bool) -> Vec<IOValue> {
    vec![
        record("check", vec![string("evidence-only"), string("pass")]),
        record("check", vec![string("no-authority-grant"), string("pass")]),
        record("check", vec![string("rollup-and-receipt-refs-verified"), string("pass")]),
        record("check", vec![
            string("all-inputs-readable"),
            string(if all_inputs_readable { "pass" } else { "fail" }),
        ]),
        record("check", vec![string("index-decision"), string(decision)]),
    ]
}

fn refs_value(refs: &BTreeSet<String>) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn divergence_counts_value(counts: &BTreeMap<String, u64>) -> IOValue {
    sequence(
        counts
            .iter()
            .map(|(kind, count)| record("divergence-count", vec![string(kind), u64_value(*count)]))
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
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string_value(&fields[0], label)
}

fn record_u64_value(value: &Value<IOValue>, label: &'static str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a u64")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("{label} out of range: {error}")))
}

fn record_ref_list_value(value: &Value<IOValue>, label: &'static str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("{label} must be a sequence")))?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        let reference = required_string_value(item, label)?;
        validate_content_ref(&reference)?;
        refs.push(reference);
    }
    Ok(refs)
}

fn record_divergence_counts_value(value: &Value<IOValue>) -> Result<BTreeMap<String, u64>> {
    let value = value_to_iovalue(value);
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
        let item = value_to_iovalue(item);
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
        validate_content_ref(reference)
    }
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::ReplayDivergenceKind;
    use super::ReplayFixtureVariant;
    use super::ReplayIndexInput;
    use super::ReplayRollupInput;
    use super::index_replay_evidence;
    use super::record_fixture_value;
    use super::rollup_replay_receipts;
    use super::verify_fixture_value;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::string;
    use crate::preserves_rail::to_text;
    use crate::runtime::PredicateDecision;
    use crate::runtime::RuntimeSnapshotAuthorityState;
    use crate::runtime::evaluate_snapshot_authority;

    #[test]
    fn replay_fixture_record_binds_identity_effects_and_final_state() {
        let fixture = record_fixture_value().expect("fixture record");
        assert!(fixture.record_ref.starts_with("blake3:"));
        assert!(fixture.identity_ref.starts_with("blake3:"));
        assert!(fixture.effect_log_ref.starts_with("blake3:"));
        assert!(fixture.output_ref.starts_with("blake3:"));
        assert!(fixture.final_state_ref.starts_with("blake3:"));
        let text = to_text(&fixture.value).expect("render fixture");
        assert!(text.contains("deterministic-fixture-record-v1"));
        assert!(text.contains("deterministic-run-identity-v1"));
        assert!(text.contains("deterministic-effect-log-v1"));
    }

    #[test]
    fn unchanged_replay_passes_and_binds_output_refs() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("verify baseline");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::None);
        assert!(receipt.first_divergence.is_none());
        assert_eq!(receipt.receipt_ref, canonical_hash(&receipt.value).expect("receipt hash"));
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
            let text = to_text(&divergence).expect("render divergence");
            assert!(text.contains(expected.as_str()));
            assert!(text.contains("safe-canonical-refs-only"));
        }
    }

    #[test]
    fn replay_profile_denies_live_external_effects() {
        let receipt = verify_fixture_value(ReplayFixtureVariant::MissingRecordedEffect).expect("verify missing effect");
        assert_eq!(receipt.decision, "deny");
        assert_eq!(receipt.divergence, ReplayDivergenceKind::LiveEffect);
        let text = to_text(&receipt.value).expect("render receipt");
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
        assert_eq!(rollup.rollup_ref, canonical_hash(&rollup.value).expect("rollup hash"));
        let text = to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("deterministic-replay-rollup-v1"));
        assert!(text.contains("effect-response"));
        assert!(text.contains("individual-receipts-required"));
    }

    #[test]
    fn replay_rollup_denies_mismatched_receipt_refs_without_counting_them() {
        let pass = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("pass replay");
        let wrong_ref = canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let rollup = rollup_replay_receipts(&[ReplayRollupInput {
            expected_ref: Some(wrong_ref.clone()),
            value: pass.value,
        }])
        .expect("rollup replay receipts");
        assert_eq!(rollup.decision, "deny");
        assert_eq!(rollup.total_count, 0);
        let text = to_text(&rollup.value).expect("render rollup");
        assert!(text.contains("replay receipt ref mismatch"));
        assert!(text.contains(&wrong_ref));
        assert!(text.contains("all-inputs-readable"));
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
        assert_eq!(index.index_ref, canonical_hash(&index.value).expect("index hash"));
        let text = to_text(&index.value).expect("render index");
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
        let wrong_ref = canonical_hash(&record_fixture_value().expect("fixture").value).expect("fixture ref");
        let index = index_replay_evidence(&[ReplayIndexInput {
            expected_ref: Some(wrong_ref.clone()),
            value: rollup.value,
        }])
        .expect("index replay evidence");
        assert_eq!(index.decision, "deny");
        assert_eq!(index.total_count, 0);
        let text = to_text(&index.value).expect("render index");
        assert!(text.contains("replay index ref mismatch"));
        assert!(text.contains(&wrong_ref));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_replay_identity_scheduler_trace_and_snapshot_properties(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(10_000));
        let first = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("first baseline replay");
        let second = verify_fixture_value(ReplayFixtureVariant::Baseline).expect("second baseline replay");
        assert_eq!(first.receipt_ref, second.receipt_ref);
        assert_eq!(first.decision, "pass");
        assert_eq!(first.divergence, ReplayDivergenceKind::None);

        let trace_a = record_fixture_value().expect("first fixture record");
        let trace_b = record_fixture_value().expect("second fixture record");
        assert_eq!(trace_a.record_ref, trace_b.record_ref);
        assert_eq!(trace_a.effect_log_ref, trace_b.effect_log_ref);
        assert_eq!(trace_a.final_state_ref, trace_b.final_state_ref);
        let trace_text = to_text(&trace_a.value).expect("render fixture record");
        assert!(trace_text.contains("no-ambient-observations"));

        let variant = if salt % 2 == 0 {
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

        let snapshot_ref = canonical_hash(&string(&format!("snapshot-{salt}"))).expect("snapshot ref");
        let admitted_ref = canonical_hash(&string(&format!("admitted-{salt}"))).expect("admitted ref");
        let redacted_ref = canonical_hash(&string(&format!("redacted-{salt}"))).expect("redacted ref");
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
