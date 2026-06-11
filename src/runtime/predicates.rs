use std::collections::BTreeSet;

use preserves::IOValue;

use super::PendingTurn;
use super::RuntimeObserver;
use super::RuntimeSnapshot;
use super::RuntimeValue;
use super::TurnAction;
use crate::error::Result;
use crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

const PREDICATE_ENGINE: &str = "trellis-bounded-local";
const ASSERTION_VISIBILITY_PREDICATE: &str = "molten.trellis-runtime.assertion-visibility.v1";
const TURN_COMMIT_ROLLBACK_PREDICATE: &str = "molten.trellis-runtime.turn-commit-rollback.v1";
const PRESERVES_PATTERN_PREDICATE: &str = "molten.trellis-runtime.preserves-pattern.v1";
const OBSERVE_DELIVERY_PREDICATE: &str = "molten.trellis-runtime.observe-delivery.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateDecision {
    Pass,
    Deny,
}

impl PredicateDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Committed,
    RolledBack,
    Denied,
    Failed,
}

impl TurnOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::RolledBack => "rolled-back",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePattern {
    Exact(RuntimeValue),
    Wildcard { binding: String },
}

impl RuntimePattern {
    pub fn exact(value: RuntimeValue) -> Self {
        Self::Exact(value)
    }

    pub fn wildcard(binding: impl Into<String>) -> Self {
        Self::Wildcard {
            binding: binding.into(),
        }
    }

    fn to_value(&self) -> IOValue {
        match self {
            Self::Exact(value) => record("runtime-pattern-exact-v1", vec![
                value.as_iovalue().clone(),
                record("value-ref", vec![string(value.value_ref())]),
            ]),
            Self::Wildcard { binding } => record("runtime-pattern-wildcard-v1", vec![string(binding)]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePredicateReceipt {
    pub receipt_ref: String,
    pub predicate: String,
    pub input_ref: String,
    pub decision: PredicateDecision,
    pub state_refs: Vec<String>,
    pub checks: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionVisibilityResult {
    pub is_visible: bool,
    pub visible_owner_refs: Vec<String>,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatchResult {
    pub is_match: bool,
    pub bindings: Vec<(String, String)>,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserveDeliveryResult {
    pub delivered_assertion_refs: Vec<String>,
    pub receipt: RuntimePredicateReceipt,
}

pub fn evaluate_assertion_visibility(
    snapshot: &RuntimeSnapshot,
    assertion_value: &RuntimeValue,
    live_owner_ids: &BTreeSet<String>,
) -> Result<AssertionVisibilityResult> {
    let mut visible_owner_refs = Vec::with_capacity(snapshot.assertions.len());
    for assertion in &snapshot.assertions {
        if assertion.value == *assertion_value && live_owner_ids.contains(&assertion.actor) {
            visible_owner_refs.push(assertion.assertion_ref()?);
        }
    }
    visible_owner_refs.sort();

    let input_value = record("runtime-predicate-assertion-visibility-input-v1", vec![
        record("snapshot-ref", vec![string(snapshot.snapshot_ref()?)]),
        record("assertion-value-ref", vec![string(assertion_value.value_ref())]),
        sequence(live_owner_ids.iter().map(string).collect()),
    ]);
    let checks = vec![
        "trellis-bounded-owner-set".to_string(),
        "canonical-assertion-dedup".to_string(),
        "live-owner-filter".to_string(),
        "visibility-after-final-retraction".to_string(),
    ];
    let visible_owner_count = visible_owner_refs.len();
    let is_visible = visible_owner_count > 0;
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: ASSERTION_VISIBILITY_PREDICATE,
        input_value,
        decision: PredicateDecision::Pass,
        state_refs: vec![snapshot.snapshot_ref()?],
        checks,
        diagnostics: Vec::new(),
    })?;

    Ok(AssertionVisibilityResult {
        is_visible,
        visible_owner_refs,
        receipt,
    })
}

pub fn evaluate_turn_transition(
    before: &RuntimeSnapshot,
    turn: &PendingTurn,
    after: &RuntimeSnapshot,
    outcome: TurnOutcome,
) -> Result<RuntimePredicateReceipt> {
    let expected = match outcome {
        TurnOutcome::Committed => apply_actions_to_snapshot(before, turn),
        TurnOutcome::RolledBack | TurnOutcome::Denied | TurnOutcome::Failed => before.clone(),
    };
    let decision = if expected == *after {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let diagnostics = if decision == PredicateDecision::Pass {
        Vec::new()
    } else {
        vec!["turn-transition-state-mismatch".to_string()]
    };
    let input_value = record("runtime-predicate-turn-transition-input-v1", vec![
        record("before-ref", vec![string(before.snapshot_ref()?)]),
        record("after-ref", vec![string(after.snapshot_ref()?)]),
        string(outcome.as_str()),
        sequence(turn.actions.iter().map(action_summary_value).collect()),
    ]);
    let checks = vec![
        "trellis-bounded-turn-delta".to_string(),
        "pending-actions-invisible-before-commit".to_string(),
        "atomic-commit".to_string(),
        "rollback-preserves-committed-state".to_string(),
    ];

    build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: TURN_COMMIT_ROLLBACK_PREDICATE,
        input_value,
        decision,
        state_refs: vec![before.snapshot_ref()?, after.snapshot_ref()?],
        checks,
        diagnostics,
    })
}

pub fn evaluate_pattern_match(pattern: &RuntimePattern, value: &RuntimeValue) -> Result<PatternMatchResult> {
    let (is_match, bindings) = match pattern {
        RuntimePattern::Exact(expected) => (expected == value, Vec::new()),
        RuntimePattern::Wildcard { binding } => (true, vec![(binding.clone(), value.value_ref().to_string())]),
    };
    let decision = if is_match {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let input_value = record("runtime-predicate-pattern-input-v1", vec![
        pattern.to_value(),
        value.as_iovalue().clone(),
        record("value-ref", vec![string(value.value_ref())]),
    ]);
    let checks = vec![
        "bounded-preserves-pattern-subset".to_string(),
        "deterministic-binding-order".to_string(),
        "canonical-value-refs".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PRESERVES_PATTERN_PREDICATE,
        input_value,
        decision,
        state_refs: vec![value.value_ref().to_string()],
        checks,
        diagnostics: Vec::new(),
    })?;

    Ok(PatternMatchResult {
        is_match,
        bindings,
        receipt,
    })
}

pub fn evaluate_observe_initial_delivery(
    snapshot: &RuntimeSnapshot,
    observer: &RuntimeObserver,
) -> Result<ObserveDeliveryResult> {
    let mut delivered_assertion_refs = Vec::with_capacity(snapshot.assertions.len());
    for assertion in &snapshot.assertions {
        if assertion.value == observer.pattern {
            delivered_assertion_refs.push(assertion.assertion_ref()?);
        }
    }
    delivered_assertion_refs.sort();
    let input_value = record("runtime-predicate-observe-delivery-input-v1", vec![
        record("snapshot-ref", vec![string(snapshot.snapshot_ref()?)]),
        observer.to_value(),
        sequence(delivered_assertion_refs.iter().map(string).collect()),
    ]);
    let checks = vec![
        "trellis-bounded-current-assertion-set".to_string(),
        "observe-pattern-match".to_string(),
        "deterministic-delivery-order".to_string(),
        "matching-retraction-propagation-boundary".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: OBSERVE_DELIVERY_PREDICATE,
        input_value,
        decision: PredicateDecision::Pass,
        state_refs: vec![snapshot.snapshot_ref()?, observer.observer_ref()?],
        checks,
        diagnostics: Vec::new(),
    })?;

    Ok(ObserveDeliveryResult {
        delivered_assertion_refs,
        receipt,
    })
}

struct RuntimePredicateReceiptInput {
    predicate: &'static str,
    input_value: IOValue,
    decision: PredicateDecision,
    state_refs: Vec<String>,
    checks: Vec<String>,
    diagnostics: Vec<String>,
}

fn build_runtime_predicate_receipt(input: RuntimePredicateReceiptInput) -> Result<RuntimePredicateReceipt> {
    let input_ref = canonical_hash(&input.input_value)?;
    let value = record("runtime-predicate-receipt-v1", vec![
        string(RUNTIME_PREDICATE_RECEIPT_SCHEMA),
        string(input.predicate),
        string(PREDICATE_ENGINE),
        record("input-ref", vec![string(&input_ref)]),
        string(input.decision.as_str()),
        sequence(input.state_refs.iter().map(string).collect()),
        sequence(input.checks.iter().map(string).collect()),
        sequence(input.diagnostics.iter().map(string).collect()),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(RuntimePredicateReceipt {
        receipt_ref,
        predicate: input.predicate.to_string(),
        input_ref,
        decision: input.decision,
        state_refs: input.state_refs,
        checks: input.checks,
        diagnostics: input.diagnostics,
        value,
    })
}

fn action_summary_value(action: &TurnAction) -> IOValue {
    match action {
        TurnAction::Send(message) => record("turn-action-send-v1", vec![message.to_value()]),
        TurnAction::Observe(observer) => record("turn-action-observe-v1", vec![observer.to_value()]),
        TurnAction::Assert(assertion) => record("turn-action-assert-v1", vec![assertion.to_value()]),
        TurnAction::Retract(assertion) => record("turn-action-retract-v1", vec![assertion.to_value()]),
    }
}

fn apply_actions_to_snapshot(before: &RuntimeSnapshot, turn: &PendingTurn) -> RuntimeSnapshot {
    let mut after = before.clone();
    for action in &turn.actions {
        match action {
            TurnAction::Send(message) => {
                after.messages.insert(message.clone());
            }
            TurnAction::Observe(observer) => {
                after.observers.insert(observer.clone());
            }
            TurnAction::Assert(assertion) => {
                after.assertions.insert(assertion.clone());
            }
            TurnAction::Retract(assertion) => {
                after.assertions.remove(assertion);
            }
        }
    }
    after
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::PredicateDecision;
    use super::RuntimePattern;
    use super::TurnOutcome;
    use super::evaluate_assertion_visibility;
    use super::evaluate_observe_initial_delivery;
    use super::evaluate_pattern_match;
    use super::evaluate_turn_transition;
    use crate::preserves_rail::validate_content_ref;
    use crate::runtime::RuntimeObserver;
    use crate::runtime::RuntimeState;
    use crate::runtime::RuntimeStep;
    use crate::runtime::RuntimeValue;

    #[test]
    fn assertion_visibility_preserves_duplicates_until_final_owner() {
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-b".into(),
            value: ready.clone(),
        });
        let mut live = BTreeSet::new();
        live.insert("owner-a".to_string());
        live.insert("owner-b".to_string());
        let both = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert!(both.is_visible);
        assert_eq!(both.visible_owner_refs.len(), 2);
        validate_content_ref(&both.receipt.receipt_ref).expect("receipt ref");

        state.apply_step(&RuntimeStep::Retract {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        let one = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert!(one.is_visible);
        assert_eq!(one.visible_owner_refs.len(), 1);

        state.apply_step(&RuntimeStep::Retract {
            actor: "owner-b".into(),
            value: ready.clone(),
        });
        let none = evaluate_assertion_visibility(&state.snapshot(), &ready, &live).expect("visibility");
        assert_eq!(none.visible_owner_refs.len(), 0);
        assert!(!none.is_visible);
    }

    #[test]
    fn turn_commit_and_rollback_predicates_bind_state_transition() {
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        let mut state = RuntimeState::new(1);
        let step = RuntimeStep::Assert {
            actor: "svc".into(),
            value,
        };
        let before = state.snapshot();
        let turn = state.begin_turn(&step);
        let rollback =
            evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Denied).expect("rollback receipt");
        assert_eq!(rollback.decision, PredicateDecision::Pass);

        let (_events, runtime_commit_receipt) =
            state.commit_turn_with_predicate_receipt(turn.clone()).expect("runtime predicate commit");
        assert_eq!(runtime_commit_receipt.decision, PredicateDecision::Pass);
        let after = state.snapshot();
        let commit = evaluate_turn_transition(&before, &turn, &after, TurnOutcome::Committed).expect("commit receipt");
        assert_eq!(commit.decision, PredicateDecision::Pass);
        let stale = evaluate_turn_transition(&before, &turn, &before, TurnOutcome::Committed).expect("stale receipt");
        assert_eq!(stale.decision, PredicateDecision::Deny);
    }

    #[test]
    fn bounded_pattern_matching_is_deterministic() {
        let value = RuntimeValue::string("service.ready").expect("runtime value");
        let exact = evaluate_pattern_match(&RuntimePattern::exact(value.clone()), &value).expect("exact match");
        assert!(exact.is_match);
        assert_eq!(exact.bindings.len(), 0);
        let wildcard = evaluate_pattern_match(&RuntimePattern::wildcard("subject"), &value).expect("wildcard");
        assert!(wildcard.is_match);
        assert_eq!(wildcard.bindings, vec![("subject".to_string(), value.value_ref().to_string())]);
    }

    #[test]
    fn observe_initial_delivery_identifies_current_visible_assertions() {
        let ready = RuntimeValue::string("service.ready").expect("runtime value");
        let other = RuntimeValue::string("service.other").expect("runtime value");
        let mut state = RuntimeState::new(1);
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        });
        state.apply_step(&RuntimeStep::Assert {
            actor: "owner-b".into(),
            value: other,
        });
        let observer = RuntimeObserver {
            actor: "watcher".to_string(),
            pattern: ready,
        };
        let result = evaluate_observe_initial_delivery(&state.snapshot(), &observer).expect("delivery");
        assert_eq!(result.delivered_assertion_refs.len(), 1);
        assert_eq!(result.receipt.decision, PredicateDecision::Pass);
    }
}
