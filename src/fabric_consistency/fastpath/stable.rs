use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::profile::DerivedQuorums;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationIdentity {
    pub command_ref: String,
    pub session_ref: String,
    pub session_sequence: u64,
    pub group_ref: String,
    pub extension_generation: u64,
    pub application_schema_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub resource_ref: String,
    pub engine_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastAcknowledgement {
    pub replica_id: String,
    pub acceleration_view: u64,
    pub base_view: u64,
    pub operation: OperationIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposerPromise {
    pub proposer_id: String,
    pub acceleration_view: u64,
    pub base_view: u64,
    pub proposal_order_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableViewAttempt {
    pub operation: OperationIdentity,
    pub original_operation: OperationIdentity,
    pub acceleration_view: u64,
    pub base_view: u64,
    pub conflict_free: bool,
    /// Admitted replica set of the view; acknowledgements from other ids are ignored.
    pub replicas: BTreeSet<String>,
    pub acknowledgements: Vec<FastAcknowledgement>,
    pub promises: Vec<ProposerPromise>,
    pub active_proposers: BTreeSet<String>,
    pub original_path_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FallbackReason {
    Conflict,
    IdentityMismatch,
    InsufficientAcknowledgements,
    MissingProposerPromise,
    MixedView,
    OriginalPathUnavailable,
    ReorderingProposer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableViewDecision {
    FastCommitted,
    OriginalOnly,
    Fallback(FallbackReason),
}

/// Exactly-once application keyed by client session operation `(session_ref, session_sequence)`.
/// Equal commands from distinct sessions are distinct operations; a retry of one session operation
/// must carry its original identity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplicationLedger {
    session_operations: BTreeMap<(String, u64), OperationIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvergenceOutcome {
    pub is_applied: bool,
    pub is_replied: bool,
    pub is_duplicate_suppressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LedgerIssue {
    /// A session operation was retried with a different command or identity.
    ConflictingSessionRetry,
}

// r[impl molten.consensus.fast_path_model.stable_view]
// r[impl molten.consensus.fast_path_model.fallback_identity]
pub fn evaluate_stable_view(attempt: &StableViewAttempt, quorums: DerivedQuorums) -> StableViewDecision {
    if attempt.operation != attempt.original_operation {
        return fallback(attempt, FallbackReason::IdentityMismatch);
    }
    if !attempt.conflict_free {
        return fallback(attempt, FallbackReason::Conflict);
    }
    let Some(acknowledged) = same_view_acknowledgers(attempt) else {
        return fallback(attempt, FallbackReason::MixedView);
    };
    if acknowledged.len() < quorums.superquorum {
        return fallback(attempt, FallbackReason::InsufficientAcknowledgements);
    }
    match validate_promises(attempt) {
        Ok(()) => StableViewDecision::FastCommitted,
        Err(reason) => fallback(attempt, reason),
    }
}

fn fallback(attempt: &StableViewAttempt, reason: FallbackReason) -> StableViewDecision {
    if attempt.original_path_available {
        StableViewDecision::Fallback(reason)
    } else {
        StableViewDecision::Fallback(FallbackReason::OriginalPathUnavailable)
    }
}

/// Distinct admitted replicas that acknowledged this exact attempt; `None` when an admitted
/// replica acknowledged another view or operation. Ids outside the admitted set never count.
fn same_view_acknowledgers(attempt: &StableViewAttempt) -> Option<BTreeSet<String>> {
    let mut replicas = BTreeSet::new();
    for acknowledgement in &attempt.acknowledgements {
        if !attempt.replicas.contains(&acknowledgement.replica_id) {
            continue;
        }
        if acknowledgement.acceleration_view != attempt.acceleration_view
            || acknowledgement.base_view != attempt.base_view
            || acknowledgement.operation != attempt.operation
        {
            return None;
        }
        replicas.insert(acknowledgement.replica_id.clone());
    }
    Some(replicas)
}

fn validate_promises(attempt: &StableViewAttempt) -> Result<(), FallbackReason> {
    let mut promised = BTreeSet::new();
    for promise in &attempt.promises {
        if promise.acceleration_view != attempt.acceleration_view || promise.base_view != attempt.base_view {
            return Err(FallbackReason::MixedView);
        }
        if !promise.proposal_order_preserved {
            return Err(FallbackReason::ReorderingProposer);
        }
        promised.insert(promise.proposer_id.clone());
    }
    if promised != attempt.active_proposers {
        return Err(FallbackReason::MissingProposerPromise);
    }
    Ok(())
}

impl ApplicationLedger {
    pub fn new() -> Self {
        Self {
            session_operations: BTreeMap::new(),
        }
    }

    /// Apply and reply once per session operation. A retry with the same identity is suppressed;
    /// a retry that changes the command or any other identity field is rejected.
    // r[impl molten.consensus.fast_path_model.fallback_identity]
    pub fn converge(&mut self, operation: &OperationIdentity) -> Result<ConvergenceOutcome, LedgerIssue> {
        let key = (operation.session_ref.clone(), operation.session_sequence);
        if let Some(original) = self.session_operations.get(&key) {
            if original != operation {
                return Err(LedgerIssue::ConflictingSessionRetry);
            }
            return Ok(ConvergenceOutcome {
                is_applied: false,
                is_replied: false,
                is_duplicate_suppressed: true,
            });
        }
        self.session_operations.insert(key, operation.clone());
        Ok(ConvergenceOutcome {
            is_applied: true,
            is_replied: true,
            is_duplicate_suppressed: false,
        })
    }

    pub fn applied_count(&self) -> usize {
        self.session_operations.len()
    }
}
