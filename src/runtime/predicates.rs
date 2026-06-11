use std::collections::BTreeSet;

use preserves::IOValue;

use super::PendingTurn;
use super::RuntimeObserver;
use super::RuntimeSnapshot;
use super::RuntimeValue;
use super::TurnAction;
use crate::error::Result;
use crate::preserves_rail::RUNTIME_PREDICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;

const PREDICATE_ENGINE: &str = "trellis-bounded-local";
const ASSERTION_VISIBILITY_PREDICATE: &str = "molten.trellis-runtime.assertion-visibility.v1";
const TURN_COMMIT_ROLLBACK_PREDICATE: &str = "molten.trellis-runtime.turn-commit-rollback.v1";
const PRESERVES_PATTERN_PREDICATE: &str = "molten.trellis-runtime.preserves-pattern.v1";
const OBSERVE_DELIVERY_PREDICATE: &str = "molten.trellis-runtime.observe-delivery.v1";
const PROMISE_STATE_PREDICATE: &str = "molten.trellis-runtime.promise-state.v1";
const PROMISE_PIPELINE_PREDICATE: &str = "molten.trellis-runtime.promise-pipeline.v1";
const REVOCATION_CLEANUP_PREDICATE: &str = "molten.trellis-runtime.revocation-cleanup.v1";
const ACTORMAP_TRANSACTION_PREDICATE: &str = "molten.trellis-runtime.actormap-transaction.v1";
const NEAR_FAR_REFS_PREDICATE: &str = "molten.trellis-runtime.near-far-refs.v1";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePromiseStatus {
    Pending,
    Resolved,
    Broken,
    Cancelled,
    TimedOut,
}

impl RuntimePromiseStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Broken => "broken",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed-out",
        }
    }

    fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromiseState {
    pub promise_id: String,
    pub status: RuntimePromiseStatus,
    pub value_ref: Option<String>,
    pub reason: Option<String>,
    pub caused_by: Vec<String>,
}

impl RuntimePromiseState {
    pub fn pending(promise_id: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Pending,
            value_ref: None,
            reason: None,
            caused_by: Vec::new(),
        }
    }

    pub fn resolved(promise_id: impl Into<String>, value_ref: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Resolved,
            value_ref: Some(value_ref.into()),
            reason: None,
            caused_by: Vec::new(),
        }
    }

    pub fn broken(promise_id: impl Into<String>, reason: impl Into<String>, caused_by: Vec<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Broken,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by,
        }
    }

    pub fn cancelled(promise_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Cancelled,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by: Vec::new(),
        }
    }

    pub fn timed_out(promise_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::TimedOut,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by: Vec::new(),
        }
    }

    pub fn promise_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IOValue {
        record("runtime-promise-state-v1", vec![
            string(&self.promise_id),
            string(self.status.as_str()),
            optional_string_value(self.value_ref.as_deref()),
            optional_string_value(self.reason.as_deref()),
            sequence(self.caused_by.iter().map(string).collect()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseStateResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromisePipelineEntry {
    pub sequence: u64,
    pub target_ref: String,
    pub operation: String,
}

impl RuntimePromisePipelineEntry {
    pub fn new(sequence: u64, target_ref: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            sequence,
            target_ref: target_ref.into(),
            operation: operation.into(),
        }
    }

    fn to_value(&self) -> IOValue {
        record("runtime-promise-pipeline-entry-v1", vec![
            crate::preserves_rail::u64_value(self.sequence),
            record("target-ref", vec![string(&self.target_ref)]),
            string(&self.operation),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromisePipelineState {
    pub source: RuntimePromiseState,
    pub max_queue: u64,
    pub entries: Vec<RuntimePromisePipelineEntry>,
}

impl RuntimePromisePipelineState {
    pub fn new(source: RuntimePromiseState, max_queue: u64, entries: Vec<RuntimePromisePipelineEntry>) -> Self {
        Self {
            source,
            max_queue,
            entries,
        }
    }

    pub fn pipeline_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IOValue {
        record("runtime-promise-pipeline-state-v1", vec![
            self.source.to_value(),
            crate::preserves_rail::u64_value(self.max_queue),
            sequence(self.entries.iter().map(RuntimePromisePipelineEntry::to_value).collect()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromisePipelineResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRevocationCleanupState {
    pub revoked_refs: Vec<String>,
    pub attempted_use_refs: Vec<String>,
    pub remaining_assertion_refs: Vec<String>,
    pub remaining_subscription_refs: Vec<String>,
    pub remaining_pending_call_refs: Vec<String>,
    pub remaining_child_refs: Vec<String>,
}

impl RuntimeRevocationCleanupState {
    pub fn cleanup_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IOValue {
        record("runtime-revocation-cleanup-state-v1", vec![
            ref_list_value("revoked-refs", &self.revoked_refs),
            ref_list_value("attempted-use-refs", &self.attempted_use_refs),
            ref_list_value("remaining-assertion-refs", &self.remaining_assertion_refs),
            ref_list_value("remaining-subscription-refs", &self.remaining_subscription_refs),
            ref_list_value("remaining-pending-call-refs", &self.remaining_pending_call_refs),
            ref_list_value("remaining-child-refs", &self.remaining_child_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationCleanupResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeActormapTransactionOutcome {
    Committed,
    RolledBack,
}

impl RuntimeActormapTransactionOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::RolledBack => "rolled-back",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeActormapTransactionState {
    pub outcome: RuntimeActormapTransactionOutcome,
    pub before_object_refs: Vec<String>,
    pub after_object_refs: Vec<String>,
    pub spawned_object_refs: Vec<String>,
    pub removed_object_refs: Vec<String>,
    pub visible_object_refs: Vec<String>,
    pub used_object_refs: Vec<String>,
}

impl RuntimeActormapTransactionState {
    pub fn transaction_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IOValue {
        record("runtime-actormap-transaction-state-v1", vec![
            string(self.outcome.as_str()),
            ref_list_value("before-object-refs", &self.before_object_refs),
            ref_list_value("after-object-refs", &self.after_object_refs),
            ref_list_value("spawned-object-refs", &self.spawned_object_refs),
            ref_list_value("removed-object-refs", &self.removed_object_refs),
            ref_list_value("visible-object-refs", &self.visible_object_refs),
            ref_list_value("used-object-refs", &self.used_object_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActormapTransactionResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeReferenceKind {
    Near,
    Far,
}

impl RuntimeReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::Far => "far",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeReferenceCallMode {
    Synchronous,
    Asynchronous,
}

impl RuntimeReferenceCallMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Synchronous => "synchronous",
            Self::Asynchronous => "asynchronous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNearFarRefState {
    pub reference_ref: String,
    pub reference_kind: RuntimeReferenceKind,
    pub is_live: bool,
    pub caller_vat_id: String,
    pub target_vat_id: String,
    pub call_mode: RuntimeReferenceCallMode,
}

impl RuntimeNearFarRefState {
    pub fn call_ref(&self) -> Result<String> {
        canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IOValue {
        record("runtime-near-far-ref-state-v1", vec![
            record("reference-ref", vec![string(&self.reference_ref)]),
            string(self.reference_kind.as_str()),
            bool_value(self.is_live),
            string(&self.caller_vat_id),
            string(&self.target_vat_id),
            string(self.call_mode.as_str()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearFarRefsResult {
    pub is_allowed: bool,
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

pub fn evaluate_promise_state_transition(
    before: &RuntimePromiseState,
    after: &RuntimePromiseState,
) -> Result<PromiseStateResult> {
    let mut diagnostics = validate_promise_shape(before, "before");
    diagnostics.extend(validate_promise_shape(after, "after"));
    if before.promise_id != after.promise_id {
        diagnostics.push("promise-id-mismatch".to_string());
    }
    if before.status.is_terminal() && before != after {
        diagnostics.push("terminal-promise-state-changed".to_string());
    }
    if before.status == RuntimePromiseStatus::Pending
        && after.status == RuntimePromiseStatus::Pending
        && before != after
    {
        diagnostics.push("pending-promise-mutated-without-resolution".to_string());
    }
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let before_ref = before.promise_ref()?;
    let after_ref = after.promise_ref()?;
    let input_value = record("runtime-predicate-promise-state-input-v1", vec![
        record("before-ref", vec![string(&before_ref)]),
        record("after-ref", vec![string(&after_ref)]),
        before.to_value(),
        after.to_value(),
    ]);
    let checks = vec![
        "bounded-promise-state-machine".to_string(),
        "terminal-state-immutability".to_string(),
        "resolved-value-ref-canonical".to_string(),
        "causal-failure-refs-canonical".to_string(),
        "cancel-timeout-reason-required".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PROMISE_STATE_PREDICATE,
        input_value,
        decision,
        state_refs: vec![before_ref, after_ref],
        checks,
        diagnostics,
    })?;

    Ok(PromiseStateResult { is_allowed, receipt })
}

pub fn evaluate_promise_pipeline(state: &RuntimePromisePipelineState) -> Result<PromisePipelineResult> {
    let diagnostics = validate_promise_pipeline(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let pipeline_ref = state.pipeline_ref()?;
    let source_ref = state.source.promise_ref()?;
    let input_value = record("runtime-predicate-promise-pipeline-input-v1", vec![
        record("pipeline-ref", vec![string(&pipeline_ref)]),
        record("source-promise-ref", vec![string(&source_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "bounded-promise-pipeline-queue".to_string(),
        "pending-source-allows-forwarding".to_string(),
        "terminal-source-cleans-pipeline".to_string(),
        "deterministic-forwarding-order".to_string(),
        "pipeline-target-refs-canonical".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: PROMISE_PIPELINE_PREDICATE,
        input_value,
        decision,
        state_refs: vec![pipeline_ref, source_ref],
        checks,
        diagnostics,
    })?;

    Ok(PromisePipelineResult { is_allowed, receipt })
}

pub fn evaluate_revocation_cleanup(state: &RuntimeRevocationCleanupState) -> Result<RevocationCleanupResult> {
    let diagnostics = validate_revocation_cleanup(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let cleanup_ref = state.cleanup_ref()?;
    let input_value = record("runtime-predicate-revocation-cleanup-input-v1", vec![
        record("cleanup-ref", vec![string(&cleanup_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "revoked-refs-canonical".to_string(),
        "revoked-refs-deny-future-use".to_string(),
        "dependent-assertions-cleaned".to_string(),
        "dependent-subscriptions-cleaned".to_string(),
        "pending-calls-cleaned".to_string(),
        "child-refs-cleaned".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: REVOCATION_CLEANUP_PREDICATE,
        input_value,
        decision,
        state_refs: vec![cleanup_ref],
        checks,
        diagnostics,
    })?;

    Ok(RevocationCleanupResult { is_allowed, receipt })
}

pub fn evaluate_actormap_transaction(state: &RuntimeActormapTransactionState) -> Result<ActormapTransactionResult> {
    let diagnostics = validate_actormap_transaction(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let transaction_ref = state.transaction_ref()?;
    let input_value = record("runtime-predicate-actormap-transaction-input-v1", vec![
        record("transaction-ref", vec![string(&transaction_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "actormap-refs-canonical".to_string(),
        "actormap-delta-commit".to_string(),
        "actormap-rollback-preserves-state".to_string(),
        "spawned-object-visibility-after-commit".to_string(),
        "removed-object-invalidation".to_string(),
    ];
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: ACTORMAP_TRANSACTION_PREDICATE,
        input_value,
        decision,
        state_refs: vec![transaction_ref],
        checks,
        diagnostics,
    })?;

    Ok(ActormapTransactionResult { is_allowed, receipt })
}

pub fn evaluate_near_far_refs(state: &RuntimeNearFarRefState) -> Result<NearFarRefsResult> {
    let diagnostics = validate_near_far_refs(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let call_ref = state.call_ref()?;
    let input_value = record("runtime-predicate-near-far-refs-input-v1", vec![
        record("call-ref", vec![string(&call_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "reference-ref-canonical".to_string(),
        "live-reference-required".to_string(),
        "near-ref-synchronous-same-vat".to_string(),
        "far-ref-asynchronous-only".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(2);
    state_refs.push(call_ref);
    if validate_content_ref(&state.reference_ref).is_ok() {
        state_refs.push(state.reference_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: NEAR_FAR_REFS_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(NearFarRefsResult { is_allowed, receipt })
}

fn validate_near_far_refs(state: &RuntimeNearFarRefState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if validate_content_ref(&state.reference_ref).is_err() {
        diagnostics.push("reference-ref-noncanonical".to_string());
    }
    if state.caller_vat_id.is_empty() {
        diagnostics.push("caller-vat-id-empty".to_string());
    }
    if state.target_vat_id.is_empty() {
        diagnostics.push("target-vat-id-empty".to_string());
    }
    if !state.is_live {
        diagnostics.push("reference-not-live".to_string());
    }

    let is_same_vat = state.caller_vat_id == state.target_vat_id;
    match state.reference_kind {
        RuntimeReferenceKind::Near => {
            if !is_same_vat {
                diagnostics.push("near-ref-cross-vat".to_string());
            }
            if matches!(state.call_mode, RuntimeReferenceCallMode::Synchronous) && !is_same_vat {
                diagnostics.push("synchronous-call-not-live-same-vat-near-ref".to_string());
            }
        }
        RuntimeReferenceKind::Far => {
            if matches!(state.call_mode, RuntimeReferenceCallMode::Synchronous) {
                diagnostics.push("far-ref-synchronous-call-denied".to_string());
            }
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_promise_pipeline(state: &RuntimePromisePipelineState) -> Vec<String> {
    let mut diagnostics = validate_promise_shape(&state.source, "source");
    if state.max_queue == 0 && !state.entries.is_empty() {
        diagnostics.push("pipeline-queue-nonempty-with-zero-bound".to_string());
    }
    if (state.entries.len() as u64) > state.max_queue {
        diagnostics.push("pipeline-queue-bound-exceeded".to_string());
    }
    if state.source.status.is_terminal() && !state.entries.is_empty() {
        diagnostics.push("terminal-promise-pipeline-not-cleaned".to_string());
    }
    let mut previous_sequence = None;
    let mut seen_sequences = BTreeSet::new();
    for entry in state.entries.as_slice() {
        if !seen_sequences.insert(entry.sequence) {
            diagnostics.push("pipeline-forwarding-sequence-duplicate".to_string());
        }
        if let Some(previous) = previous_sequence
            && entry.sequence <= previous
        {
            diagnostics.push("pipeline-forwarding-order-violation".to_string());
        }
        previous_sequence = Some(entry.sequence);
        if entry.operation.is_empty() {
            diagnostics.push("pipeline-operation-empty".to_string());
        }
        if validate_content_ref(&entry.target_ref).is_err() {
            diagnostics.push("pipeline-target-ref-noncanonical".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_revocation_cleanup(state: &RuntimeRevocationCleanupState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(16);
    diagnostics.extend(validate_sorted_content_refs(&state.revoked_refs, "revocation", "revoked"));
    diagnostics.extend(validate_sorted_content_refs(&state.attempted_use_refs, "revocation", "attempted-use"));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_assertion_refs,
        "revocation",
        "remaining-assertion",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_subscription_refs,
        "revocation",
        "remaining-subscription",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.remaining_pending_call_refs,
        "revocation",
        "remaining-pending-call",
    ));
    diagnostics.extend(validate_sorted_content_refs(&state.remaining_child_refs, "revocation", "remaining-child"));

    let revoked_refs: BTreeSet<&str> = state.revoked_refs.as_slice().iter().map(String::as_str).collect();
    if has_revoked_intersection(&revoked_refs, &state.attempted_use_refs) {
        diagnostics.push("revoked-ref-used-after-revocation".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_assertion_refs) {
        diagnostics.push("revoked-dependent-assertion-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_subscription_refs) {
        diagnostics.push("revoked-dependent-subscription-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_pending_call_refs) {
        diagnostics.push("revoked-pending-call-not-cleaned".to_string());
    }
    if has_revoked_intersection(&revoked_refs, &state.remaining_child_refs) {
        diagnostics.push("revoked-child-ref-not-cleaned".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn has_revoked_intersection(revoked_refs: &BTreeSet<&str>, refs: &[String]) -> bool {
    for reference in refs {
        if revoked_refs.contains(reference.as_str()) {
            return true;
        }
    }
    false
}

fn validate_actormap_transaction(state: &RuntimeActormapTransactionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(24);
    diagnostics.extend(validate_sorted_content_refs(&state.before_object_refs, "actormap", "before-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.after_object_refs, "actormap", "after-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.spawned_object_refs, "actormap", "spawned-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.removed_object_refs, "actormap", "removed-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.visible_object_refs, "actormap", "visible-object"));
    diagnostics.extend(validate_sorted_content_refs(&state.used_object_refs, "actormap", "used-object"));

    let before_refs = string_set(&state.before_object_refs);
    let after_refs = string_set(&state.after_object_refs);
    let spawned_refs = string_set(&state.spawned_object_refs);
    let removed_refs = string_set(&state.removed_object_refs);
    let visible_refs = string_set(&state.visible_object_refs);
    let used_refs = string_set(&state.used_object_refs);

    if has_set_intersection(&spawned_refs, &before_refs) {
        diagnostics.push("spawned-object-already-existed".to_string());
    }
    if !is_subset(&removed_refs, &before_refs) {
        diagnostics.push("removed-object-missing-before".to_string());
    }

    match state.outcome {
        RuntimeActormapTransactionOutcome::Committed => {
            let mut expected_after = before_refs.clone();
            for removed in &removed_refs {
                expected_after.remove(*removed);
            }
            for spawned in &spawned_refs {
                expected_after.insert(*spawned);
            }
            if expected_after != after_refs {
                diagnostics.push("actormap-commit-delta-mismatch".to_string());
            }
            if !is_subset(&spawned_refs, &after_refs) {
                diagnostics.push("spawned-object-missing-after-commit".to_string());
            }
            if !is_subset(&spawned_refs, &visible_refs) {
                diagnostics.push("spawned-object-not-visible-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &after_refs) {
                diagnostics.push("removed-object-present-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &visible_refs) {
                diagnostics.push("removed-object-visible-after-commit".to_string());
            }
            if has_set_intersection(&removed_refs, &used_refs) {
                diagnostics.push("removed-object-used-after-removal".to_string());
            }
        }
        RuntimeActormapTransactionOutcome::RolledBack => {
            if before_refs != after_refs {
                diagnostics.push("actormap-rollback-state-changed".to_string());
            }
            if has_set_intersection(&spawned_refs, &visible_refs) {
                diagnostics.push("spawned-object-visible-after-rollback".to_string());
            }
            if has_set_intersection(&spawned_refs, &used_refs) {
                diagnostics.push("spawned-object-used-after-rollback".to_string());
            }
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn string_set(refs: &[String]) -> BTreeSet<&str> {
    refs.iter().map(String::as_str).collect()
}

fn is_subset(left: &BTreeSet<&str>, right: &BTreeSet<&str>) -> bool {
    for item in left {
        if !right.contains(item) {
            return false;
        }
    }
    true
}

fn has_set_intersection(left: &BTreeSet<&str>, right: &BTreeSet<&str>) -> bool {
    for item in left {
        if right.contains(item) {
            return true;
        }
    }
    false
}

fn validate_promise_shape(state: &RuntimePromiseState, label: &str) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if state.promise_id.is_empty() {
        diagnostics.push(format!("{label}-promise-id-empty"));
    }
    match state.status {
        RuntimePromiseStatus::Pending => {
            if state.value_ref.is_some() || state.reason.is_some() || !state.caused_by.is_empty() {
                diagnostics.push(format!("{label}-pending-promise-has-terminal-data"));
            }
        }
        RuntimePromiseStatus::Resolved => {
            if !state.caused_by.is_empty() || state.reason.is_some() {
                diagnostics.push(format!("{label}-resolved-promise-has-failure-data"));
            }
            match state.value_ref.as_deref() {
                Some(value_ref) if validate_content_ref(value_ref).is_ok() => {}
                Some(_) => diagnostics.push(format!("{label}-resolved-value-ref-noncanonical")),
                None => diagnostics.push(format!("{label}-resolved-value-ref-missing")),
            }
        }
        RuntimePromiseStatus::Broken => {
            if state.value_ref.is_some() {
                diagnostics.push(format!("{label}-broken-promise-has-value"));
            }
            if state.reason.as_deref().is_none_or(str::is_empty) {
                diagnostics.push(format!("{label}-broken-reason-missing"));
            }
            diagnostics.extend(validate_sorted_content_refs(&state.caused_by, label, "causal-failure"));
        }
        RuntimePromiseStatus::Cancelled | RuntimePromiseStatus::TimedOut => {
            if state.value_ref.is_some() || !state.caused_by.is_empty() {
                diagnostics.push(format!("{label}-cancel-timeout-has-resolution-data"));
            }
            if state.reason.as_deref().is_none_or(str::is_empty) {
                diagnostics.push(format!("{label}-cancel-timeout-reason-missing"));
            }
        }
    }
    diagnostics
}

fn ref_list_value(label: &'static str, refs: &[String]) -> IOValue {
    record(label, vec![sequence(refs.iter().map(string).collect())])
}

fn validate_sorted_content_refs(refs: &[String], label: &str, field: &str) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(refs.len() + 1);
    for reference in refs {
        if validate_content_ref(reference).is_err() {
            diagnostics.push(format!("{label}-{field}-ref-noncanonical"));
        }
    }
    let mut sorted_refs = refs.to_vec();
    sorted_refs.sort();
    sorted_refs.dedup();
    if sorted_refs != refs {
        diagnostics.push(format!("{label}-{field}-refs-not-sorted-unique"));
    }
    diagnostics
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

fn optional_string_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
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
    use super::RuntimeActormapTransactionOutcome;
    use super::RuntimeActormapTransactionState;
    use super::RuntimeNearFarRefState;
    use super::RuntimePattern;
    use super::RuntimePromisePipelineEntry;
    use super::RuntimePromisePipelineState;
    use super::RuntimePromiseState;
    use super::RuntimeReferenceCallMode;
    use super::RuntimeReferenceKind;
    use super::RuntimeRevocationCleanupState;
    use super::TurnOutcome;
    use super::evaluate_actormap_transaction;
    use super::evaluate_assertion_visibility;
    use super::evaluate_near_far_refs;
    use super::evaluate_observe_initial_delivery;
    use super::evaluate_pattern_match;
    use super::evaluate_promise_pipeline;
    use super::evaluate_promise_state_transition;
    use super::evaluate_revocation_cleanup;
    use super::evaluate_turn_transition;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::string;
    use crate::preserves_rail::validate_content_ref;
    use crate::runtime::RuntimeObserver;
    use crate::runtime::RuntimeState;
    use crate::runtime::RuntimeStep;
    use crate::runtime::RuntimeValue;

    fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
        refs.sort();
        refs
    }

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

    #[test]
    fn promise_state_predicate_enforces_terminal_and_causal_rules() {
        let value_ref = canonical_hash(&string("resolved-value")).expect("value ref");
        let cause_ref = canonical_hash(&string("upstream-promise")).expect("cause ref");
        let pending = RuntimePromiseState::pending("promise-1");
        let resolved = RuntimePromiseState::resolved("promise-1", value_ref.clone());
        let pass = evaluate_promise_state_transition(&pending, &resolved).expect("promise transition");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let changed_terminal = RuntimePromiseState::broken("promise-1", "late failure", vec![cause_ref.clone()]);
        let terminal = evaluate_promise_state_transition(&resolved, &changed_terminal).expect("terminal transition");
        assert!(!terminal.is_allowed);
        assert_eq!(terminal.receipt.decision, PredicateDecision::Deny);
        assert!(terminal.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "terminal-promise-state-changed"));

        let mut unsorted_causes = vec![cause_ref, canonical_hash(&string("aaa")).expect("second cause")];
        unsorted_causes.sort();
        unsorted_causes.reverse();
        let unsorted_broken = RuntimePromiseState::broken("promise-2", "causal failure", unsorted_causes);
        let causal = evaluate_promise_state_transition(&RuntimePromiseState::pending("promise-2"), &unsorted_broken)
            .expect("causal transition");
        assert!(!causal.is_allowed);
        assert!(
            causal
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "after-causal-failure-refs-not-sorted-unique")
        );
    }

    #[test]
    fn promise_pipeline_predicate_bounds_order_and_cleanup() {
        let target_a = canonical_hash(&string("target-a")).expect("target a");
        let target_b = canonical_hash(&string("target-b")).expect("target b");
        let pending = RuntimePromiseState::pending("promise-pipeline");
        let pipeline = RuntimePromisePipelineState::new(pending.clone(), 2, vec![
            RuntimePromisePipelineEntry::new(1, target_a.clone(), "get:field"),
            RuntimePromisePipelineEntry::new(2, target_b.clone(), "call:method"),
        ]);
        let pass = evaluate_promise_pipeline(&pipeline).expect("pipeline predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let over_bound = RuntimePromisePipelineState::new(pending, 1, vec![
            RuntimePromisePipelineEntry::new(2, target_a.clone(), "second"),
            RuntimePromisePipelineEntry::new(1, "not-a-ref", "first"),
        ]);
        let denied = evaluate_promise_pipeline(&over_bound).expect("denied pipeline predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "pipeline-queue-bound-exceeded"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "pipeline-forwarding-order-violation")
        );
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "pipeline-target-ref-noncanonical"));

        let resolved = RuntimePromiseState::resolved("promise-pipeline", target_b);
        let stale = RuntimePromisePipelineState::new(resolved, 2, vec![RuntimePromisePipelineEntry::new(
            3,
            target_a,
            "late-forward",
        )]);
        let cleanup = evaluate_promise_pipeline(&stale).expect("cleanup predicate");
        assert!(!cleanup.is_allowed);
        assert!(
            cleanup
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "terminal-promise-pipeline-not-cleaned")
        );
    }

    #[test]
    fn revocation_cleanup_predicate_denies_future_use_and_requires_cleanup() {
        let revoked = canonical_hash(&string("revoked-ref")).expect("revoked ref");
        let live_assertion = canonical_hash(&string("live-assertion")).expect("live assertion");
        let live_subscription = canonical_hash(&string("live-subscription")).expect("live subscription");
        let live_call = canonical_hash(&string("live-call")).expect("live call");
        let live_child = canonical_hash(&string("live-child")).expect("live child");
        let pass_state = RuntimeRevocationCleanupState {
            revoked_refs: vec![revoked.clone()],
            attempted_use_refs: Vec::new(),
            remaining_assertion_refs: vec![live_assertion],
            remaining_subscription_refs: vec![live_subscription],
            remaining_pending_call_refs: vec![live_call],
            remaining_child_refs: vec![live_child],
        };
        let pass = evaluate_revocation_cleanup(&pass_state).expect("revocation cleanup predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let denied_state = RuntimeRevocationCleanupState {
            revoked_refs: vec![revoked.clone()],
            attempted_use_refs: vec![revoked.clone()],
            remaining_assertion_refs: vec![revoked.clone()],
            remaining_subscription_refs: vec![revoked.clone()],
            remaining_pending_call_refs: vec![revoked.clone()],
            remaining_child_refs: vec![revoked],
        };
        let denied = evaluate_revocation_cleanup(&denied_state).expect("denied cleanup predicate");
        assert!(!denied.is_allowed);
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-ref-used-after-revocation")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-dependent-assertion-not-cleaned")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "revoked-dependent-subscription-not-cleaned")
        );
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "revoked-pending-call-not-cleaned"));
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "revoked-child-ref-not-cleaned"));
    }

    #[test]
    fn near_far_refs_predicate_denies_dead_far_sync_and_cross_vat_near_refs() {
        let reference_ref = canonical_hash(&string("object-ref")).expect("reference ref");
        let sync_near = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-a".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let pass = evaluate_near_far_refs(&sync_near).expect("near/far predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let far_sync = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Far,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Synchronous,
        };
        let far_denied = evaluate_near_far_refs(&far_sync).expect("far sync predicate");
        assert!(!far_denied.is_allowed);
        assert!(
            far_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "far-ref-synchronous-call-denied")
        );

        let cross_vat_near = RuntimeNearFarRefState {
            reference_ref: reference_ref.clone(),
            reference_kind: RuntimeReferenceKind::Near,
            is_live: true,
            caller_vat_id: "vat-a".to_string(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Asynchronous,
        };
        let near_denied = evaluate_near_far_refs(&cross_vat_near).expect("cross-vat near predicate");
        assert!(!near_denied.is_allowed);
        assert!(near_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "near-ref-cross-vat"));

        let dead_ref = RuntimeNearFarRefState {
            reference_ref: "not-a-ref".to_string(),
            reference_kind: RuntimeReferenceKind::Far,
            is_live: false,
            caller_vat_id: String::new(),
            target_vat_id: "vat-b".to_string(),
            call_mode: RuntimeReferenceCallMode::Asynchronous,
        };
        let dead_denied = evaluate_near_far_refs(&dead_ref).expect("dead ref predicate");
        assert!(!dead_denied.is_allowed);
        assert!(dead_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "reference-not-live"));
        assert!(dead_denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "reference-ref-noncanonical"));
    }

    #[test]
    fn actormap_transaction_predicate_commits_rolls_back_and_invalidates_removed_objects() {
        let existing = canonical_hash(&string("existing-object")).expect("existing object");
        let spawned = canonical_hash(&string("spawned-object")).expect("spawned object");
        let removed = canonical_hash(&string("removed-object")).expect("removed object");
        let committed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
            used_object_refs: sorted_refs(vec![existing.clone(), spawned.clone()]),
        };
        let pass = evaluate_actormap_transaction(&committed).expect("actormap transaction predicate");
        assert!(pass.is_allowed);
        assert_eq!(pass.receipt.decision, PredicateDecision::Pass);
        validate_content_ref(&pass.receipt.receipt_ref).expect("receipt ref");

        let stale_removed = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            after_object_refs: sorted_refs(vec![existing.clone(), removed.clone(), spawned.clone()]),
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: vec![removed.clone()],
            visible_object_refs: sorted_refs(vec![existing.clone(), removed.clone()]),
            used_object_refs: vec![removed.clone()],
        };
        let denied = evaluate_actormap_transaction(&stale_removed).expect("denied actormap predicate");
        assert!(!denied.is_allowed);
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "actormap-commit-delta-mismatch"));
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-present-after-commit")
        );
        assert!(
            denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "removed-object-used-after-removal")
        );

        let rollback = RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::RolledBack,
            before_object_refs: vec![existing],
            after_object_refs: vec![spawned.clone()],
            spawned_object_refs: vec![spawned.clone()],
            removed_object_refs: Vec::new(),
            visible_object_refs: vec![spawned.clone()],
            used_object_refs: vec![spawned],
        };
        let rollback_denied = evaluate_actormap_transaction(&rollback).expect("rollback actormap predicate");
        assert!(!rollback_denied.is_allowed);
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "actormap-rollback-state-changed")
        );
        assert!(
            rollback_denied
                .receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "spawned-object-visible-after-rollback")
        );
    }
}
