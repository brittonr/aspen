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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplicationLedger {
    applied_commands: BTreeSet<String>,
    replied_operations: BTreeSet<(String, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvergenceOutcome {
    pub is_applied: bool,
    pub is_replied: bool,
    pub is_duplicate_suppressed: bool,
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

fn same_view_acknowledgers(attempt: &StableViewAttempt) -> Option<BTreeSet<String>> {
    let mut replicas = BTreeSet::new();
    for acknowledgement in &attempt.acknowledgements {
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
    // r[impl molten.consensus.fast_path_model.fallback_identity]
    pub fn converge(&mut self, operation: &OperationIdentity) -> ConvergenceOutcome {
        let is_applied = self.applied_commands.insert(operation.command_ref.clone());
        let is_replied = self.replied_operations.insert((operation.session_ref.clone(), operation.session_sequence));
        ConvergenceOutcome {
            is_applied,
            is_replied,
            is_duplicate_suppressed: !is_applied || !is_replied,
        }
    }

    pub fn applied_count(&self) -> usize {
        self.applied_commands.len()
    }
}
