use preserves::IOValue;

use crate::error::Result;
use crate::preserves_rail::DETERMINISTIC_EFFECT_LOG_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_FIXTURE_RECORD_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_RUN_IDENTITY_SCHEMA;
use crate::preserves_rail::DETERMINISTIC_TURN_JOURNAL_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

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

#[cfg(test)]
mod tests {
    use super::ReplayDivergenceKind;
    use super::ReplayFixtureVariant;
    use super::record_fixture_value;
    use super::verify_fixture_value;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::to_text;

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
}
