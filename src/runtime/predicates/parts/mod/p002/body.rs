
impl RuntimeSnapshotAuthorityState {
    pub fn authority_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-snapshot-authority-state-v1", vec![
            crate::preserves_rail::record("snapshot-ref", vec![crate::preserves_rail::string(&self.snapshot_ref)]),
            ref_list_value("admitted-authority-refs", &self.admitted_authority_refs),
            ref_list_value("claimed-authority-refs", &self.claimed_authority_refs),
            ref_list_value("requested-assertion-refs", &self.requested_assertion_refs),
            ref_list_value("readable-assertion-refs", &self.readable_assertion_refs),
            ref_list_value("redacted-assertion-refs", &self.redacted_assertion_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotAuthorityResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVatRollbackCleanupState {
    pub rollback_receipt_ref: String,
    pub before_snapshot_ref: String,
    pub final_snapshot_ref: String,
    pub rolled_back_refs: Vec<String>,
    pub remaining_assertion_refs: Vec<String>,
    pub remaining_observer_refs: Vec<String>,
    pub remaining_pending_call_refs: Vec<String>,
    pub remaining_authority_snapshot_refs: Vec<String>,
}

impl RuntimeVatRollbackCleanupState {
    pub fn cleanup_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-vat-rollback-cleanup-state-v1", vec![
            crate::preserves_rail::record("rollback-receipt-ref", vec![crate::preserves_rail::string(
                &self.rollback_receipt_ref,
            )]),
            crate::preserves_rail::record("before-snapshot-ref", vec![crate::preserves_rail::string(
                &self.before_snapshot_ref,
            )]),
            crate::preserves_rail::record("final-snapshot-ref", vec![crate::preserves_rail::string(
                &self.final_snapshot_ref,
            )]),
            ref_list_value("rolled-back-refs", &self.rolled_back_refs),
            ref_list_value("remaining-assertion-refs", &self.remaining_assertion_refs),
            ref_list_value("remaining-observer-refs", &self.remaining_observer_refs),
            ref_list_value("remaining-pending-call-refs", &self.remaining_pending_call_refs),
            ref_list_value("remaining-authority-snapshot-refs", &self.remaining_authority_snapshot_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRollbackCleanupResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeServiceDependenciesState {
    pub service_ref: String,
    pub demanded_service_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub ready_service_refs: Vec<String>,
    pub failed_service_refs: Vec<String>,
    pub force_run_refs: Vec<String>,
    pub restart_refs: Vec<String>,
    pub reverse_dependency_refs: Vec<String>,
    pub shutdown_refs: Vec<String>,
}

impl RuntimeServiceDependenciesState {
    pub fn dependency_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-service-dependencies-state-v1", vec![
            crate::preserves_rail::record("service-ref", vec![crate::preserves_rail::string(&self.service_ref)]),
            ref_list_value("demanded-service-refs", &self.demanded_service_refs),
            ref_list_value("dependency-refs", &self.dependency_refs),
            ref_list_value("ready-service-refs", &self.ready_service_refs),
            ref_list_value("failed-service-refs", &self.failed_service_refs),
            ref_list_value("force-run-refs", &self.force_run_refs),
            ref_list_value("restart-refs", &self.restart_refs),
            ref_list_value("reverse-dependency-refs", &self.reverse_dependency_refs),
            ref_list_value("shutdown-refs", &self.shutdown_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDependenciesResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

pub fn evaluate_assertion_visibility(
    snapshot: &RuntimeSnapshot,
    assertion_value: &RuntimeValue,
    live_owner_ids: &OrderedSet<String>,
) -> Result<AssertionVisibilityResult> {
    let mut visible_owner_refs = Vec::with_capacity(snapshot.assertions.len());
    for assertion in &snapshot.assertions {
        if assertion.value == *assertion_value && live_owner_ids.contains(&assertion.actor) {
            visible_owner_refs.push(assertion.assertion_ref()?);
        }
    }
    visible_owner_refs.sort();

    let input_value = crate::preserves_rail::record("runtime-predicate-assertion-visibility-input-v1", vec![
        crate::preserves_rail::record("snapshot-ref", vec![crate::preserves_rail::string(snapshot.snapshot_ref()?)]),
        crate::preserves_rail::record("assertion-value-ref", vec![crate::preserves_rail::string(
            assertion_value.value_ref(),
        )]),
        crate::preserves_rail::sequence(live_owner_ids.iter().map(crate::preserves_rail::string).collect()),
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
    let expected = expected_turn_snapshot(before, turn, outcome);
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
    let input_value = turn_transition_input_value(before, turn, after, outcome)?;
    let checks = vec![
        "trellis-bounded-turn-delta".to_string(),
        "pending-actions-invisible-before-commit".to_string(),
        "atomic-commit".to_string(),
        "rollback-preserves-committed-state".to_string(),
        "turn-event-refs-bound".to_string(),
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
    let input_value = crate::preserves_rail::record("runtime-predicate-pattern-input-v1", vec![
        pattern.to_value(),
        value.as_iovalue().clone(),
        crate::preserves_rail::record("value-ref", vec![crate::preserves_rail::string(value.value_ref())]),
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
    let input_value = crate::preserves_rail::record("runtime-predicate-observe-delivery-input-v1", vec![
        crate::preserves_rail::record("snapshot-ref", vec![crate::preserves_rail::string(snapshot.snapshot_ref()?)]),
        observer.to_value(),
        crate::preserves_rail::sequence(delivered_assertion_refs.iter().map(crate::preserves_rail::string).collect()),
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
    let input_value = crate::preserves_rail::record("runtime-predicate-promise-state-input-v1", vec![
        crate::preserves_rail::record("before-ref", vec![crate::preserves_rail::string(&before_ref)]),
        crate::preserves_rail::record("after-ref", vec![crate::preserves_rail::string(&after_ref)]),
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
