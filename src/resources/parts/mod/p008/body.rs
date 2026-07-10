// Resource reconciliation controllers — pure deterministic planning
// core, work queue, and effect commit receipts.
//
// Adapts the Kubernetes controller/operator reconcile loop but preserves
// Molten's functional-core / imperative-shell boundary. The core produces
// a no-op, action plan, retry plan, or denial from explicit input summaries.
//
// Type aliases and common helpers are inherited from p000.

const MAX_DEPENDENCIES: usize = 256;
const MAX_EFFECT_INTENTS: usize = 128;
const MAX_BACKOFF_ATTEMPTS: u64 = 10_000;
const MAX_COALESCE_EVENTS: usize = 64;
const _: () = assert!(MAX_DEPENDENCIES > 0);
const _: () = assert!(MAX_EFFECT_INTENTS > 0);
const _: () = assert!(MAX_BACKOFF_ATTEMPTS > 0);

// ---------------------------------------------------------------------------
// Reconcile input/output DTOs
// ---------------------------------------------------------------------------

/// Pure reconcile core input — no I/O, no ambient state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileInput {
    pub resource_ref: String,
    pub resource_type: String,
    pub generation: u64,
    pub desired_state_ref: String,
    pub observed_state_summary_ref: Option<String>,
    pub status_summary_ref: Option<String>,
    pub dependency_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub prior_plan_refs: Vec<String>,
    pub prior_effect_refs: Vec<String>,
    pub prior_status_refs: Vec<String>,
    pub retry_attempt: u64,
    pub backoff_profile: Option<String>,
}

/// The pure core produces one of these outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcilePlan {
    NoOp {
        reason: String,
        condition_candidates: Vec<String>,
    },
    ActionPlan {
        effect_intents: Vec<EffectIntent>,
        required_admission_refs: Vec<String>,
    },
    RetryPlan {
        backoff_profile: String,
        next_eligible_attempt: u64,
        diagnostics: Vec<String>,
    },
    TerminalDenial {
        diagnostics: Vec<String>,
    },
}

/// An effect intent in an action plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectIntent {
    pub kind: String,
    pub target_ref: String,
    pub payload_ref: String,
    pub admission_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Work queue DTOs
// ---------------------------------------------------------------------------

/// A coalesced work queue item for reconciliation scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkQueueItem {
    pub resource_ref: String,
    pub generation: u64,
    pub causes: Vec<String>,
    pub coalesced_event_refs: Vec<String>,
    pub retry_attempt: u64,
    pub backoff_profile: Option<String>,
    pub terminal: bool,
    pub terminal_reason: Option<String>,
}

/// Work queue scheduling decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkQueueDecision {
    pub pass: bool,
    pub item: Option<WorkQueueItem>,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Reconcile receipt
// ---------------------------------------------------------------------------

/// A reconciliation receipt binding plan, admission, effects, and status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReceipt {
    pub resource_ref: String,
    pub generation: u64,
    pub plan_ref: String,
    pub admission_refs: Vec<String>,
    pub effect_refs: Vec<String>,
    pub status_update_ref: Option<String>,
    pub canonical_ref: Option<String>,
}

/// Input validation for a reconciliation completion claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCompletionInput {
    pub resource_ref: String,
    pub claimed_generation: u64,
    pub current_generation: u64,
    pub has_admitted_plan: bool,
    pub has_effect_receipts: Vec<String>,
    pub required_effect_intents: Vec<String>,
    pub has_status_update: bool,
}

/// Result of reconcil completion validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCompletionDecision {
    pub pass: bool,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pure core: reconcile evaluation
// ---------------------------------------------------------------------------

/// Evaluate a reconcile loop for a resource.
///
/// Compares desired state to observed state and produces a plan.
/// The core never reads logs, clocks, filesystems, or adapters.
pub fn evaluate_reconcile(input: &ReconcileInput) -> Result<ReconcilePlan> {
    require_ref(&input.resource_ref, "resource ref")?;
    validate_non_empty(&input.resource_type, "resource type")?;

    if input.generation == 0 {
        return Err(MoltenError::invalid_harness(
            "resource generation must be at least 1",
        ));
    }

    require_ref(&input.desired_state_ref, "desired state ref")?;

    if input.dependency_refs.len() > MAX_DEPENDENCIES {
        return Err(MoltenError::invalid_harness(format!(
            "dependency count {} exceeds maximum {MAX_DEPENDENCIES}",
            input.dependency_refs.len(),
        )));
    }

    // Check for terminal retry exhaustion
    if input.retry_attempt > MAX_BACKOFF_ATTEMPTS {
        return Ok(ReconcilePlan::TerminalDenial {
            diagnostics: vec![format!(
                "retry budget exhausted after {} attempts",
                MAX_BACKOFF_ATTEMPTS,
            )],
        });
    }

    // If no observed state exists yet, plan creates it
    if input.observed_state_summary_ref.is_none() {
        return Ok(ReconcilePlan::ActionPlan {
            effect_intents: vec![EffectIntent {
                kind: "create".to_string(),
                target_ref: input.resource_ref.clone(),
                payload_ref: input.desired_state_ref.clone(),
                admission_refs: Vec::new(),
            }],
            required_admission_refs: Vec::new(),
        });
    }

    // Determine desired-vs-observed match
    let observed_ref = input
        .observed_state_summary_ref
        .clone()
        .unwrap_or_else(|| "blake3:unknown".to_string());

    if observed_ref == input.desired_state_ref {
        // Desired matches observed — no-op
        return Ok(ReconcilePlan::NoOp {
            reason: "desired and observed state match".to_string(),
            condition_candidates: vec!["Reconciled".to_string()],
        });
    }

    // Desired differs from observed — plan corrective action
    let effect_intents = vec![EffectIntent {
        kind: "update".to_string(),
        target_ref: input.resource_ref.clone(),
        payload_ref: input.desired_state_ref.clone(),
        admission_refs: input.authority_refs.clone(),
    }];

    Ok(ReconcilePlan::ActionPlan {
        effect_intents,
        required_admission_refs: input.authority_refs.clone(),
    })
}

// ---------------------------------------------------------------------------
// Pure core: work queue operations
// ---------------------------------------------------------------------------

/// Coalesce multiple events for the same resource generation into one queue item.
pub fn coalesce_work_queue_item(
    resource_ref: &str,
    generation: u64,
    events: &[String],
) -> Result<WorkQueueDecision> {
    require_ref(resource_ref, "resource ref")?;

    if events.is_empty() {
        return Ok(WorkQueueDecision {
            pass: false,
            item: None,
            diagnostics: vec!["no events to coalesce".to_string()],
        });
    }

    if events.len() > MAX_COALESCE_EVENTS {
        return Err(MoltenError::invalid_harness(format!(
            "coalesce event count {} exceeds maximum {MAX_COALESCE_EVENTS}",
            events.len(),
        )));
    }

    let causes: Vec<String> = events.iter().map(|e| format!("watch:{e}")).collect();

    Ok(WorkQueueDecision {
        pass: true,
        item: Some(WorkQueueItem {
            resource_ref: resource_ref.to_string(),
            generation,
            causes,
            coalesced_event_refs: events.to_vec(),
            retry_attempt: 0,
            backoff_profile: None,
            terminal: false,
            terminal_reason: None,
        }),
        diagnostics: Vec::new(),
    })
}

/// Schedule a retry for a work queue item with bounded backoff.
pub fn schedule_retry(
    item: &WorkQueueItem,
    backoff_profile: &str,
    attempt: u64,
) -> Result<WorkQueueDecision> {
    if item.terminal {
        return Ok(WorkQueueDecision {
            pass: false,
            item: None,
            diagnostics: vec![format!(
                "cannot retry terminal item: {}",
                item.terminal_reason.as_deref().unwrap_or("unknown"),
            )],
        });
    }

    validate_non_empty(backoff_profile, "backoff profile")?;

    if attempt > MAX_BACKOFF_ATTEMPTS {
        return Ok(WorkQueueDecision {
            pass: false,
            item: None,
            diagnostics: vec![format!(
                "retry attempt {attempt} exceeds maximum {MAX_BACKOFF_ATTEMPTS}",
            )],
        });
    }

    Ok(WorkQueueDecision {
        pass: true,
        item: Some(WorkQueueItem {
            resource_ref: item.resource_ref.clone(),
            generation: item.generation,
            causes: item.causes.clone(),
            coalesced_event_refs: item.coalesced_event_refs.clone(),
            retry_attempt: attempt,
            backoff_profile: Some(backoff_profile.to_string()),
            terminal: false,
            terminal_reason: None,
        }),
        diagnostics: Vec::new(),
    })
}

/// Validate that a reconciliation completion claim is valid.
pub fn validate_reconcile_completion(
    input: &ReconcileCompletionInput,
) -> ReconcileCompletionDecision {
    let mut diagnostics = Vec::new();
    let mut pass = true;

    // Generation must match
    if input.claimed_generation != input.current_generation {
        pass = false;
        diagnostics.push(format!(
            "stale generation: claimed {} but current is {}",
            input.claimed_generation, input.current_generation,
        ));
    }

    // Must have an admitted plan
    if !input.has_admitted_plan {
        pass = false;
        diagnostics.push("no admitted plan for reconciliation".to_string());
    }

    // Every required effect intent must have a receipt
    for required in &input.required_effect_intents {
        if !input
            .has_effect_receipts
            .iter()
            .any(|receipt| receipt.contains(required))
        {
            pass = false;
            diagnostics.push(format!("missing effect receipt for: {required}"));
        }
    }

    // Must have status update
    if !input.has_status_update {
        pass = false;
        diagnostics.push("status update required for reconciliation success".to_string());
    }

    ReconcileCompletionDecision { pass, diagnostics }
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn reconcile_receipt_to_value(receipt: &ReconcileReceipt) -> IoValue {
    record("reconcile-receipt-v1", vec![
        string(&receipt.resource_ref),
        u64_value(receipt.generation),
        string(&receipt.plan_ref),
        refs_sequence(&receipt.admission_refs),
        refs_sequence(&receipt.effect_refs),
        optional_ref_value(receipt.status_update_ref.as_deref()),
        optional_ref_value(receipt.canonical_ref.as_deref()),
    ])
}