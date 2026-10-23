// The reference extension's local protocol rules.
//
// One pure function decides which local rules a step violates. The simulation
// oracle applies it to decoded projections; every executor applies it as a
// guard to its own states. A violation is a typed invariant result, distinct
// from the normal admission errors of `ReferenceServiceIssue`.

use super::*;
use crate::fabric::ReferenceSystemKind;
use crate::protocol_oracle::ProtocolProjection;
use crate::protocol_oracle::ProtocolSemantics;
use crate::protocol_oracle::TransitionClass;

const TRANSACTION_VERSION_MONOTONIC: &str = "transaction-version-monotonic";
const CONFLICT_DOES_NOT_MUTATE: &str = "conflict-does-not-mutate";
const LOG_OFFSETS_CONTIGUOUS: &str = "log-offsets-contiguous";
const RETENTION_FOLLOWS_REPLICATION: &str = "retention-follows-replication";
const SINGLE_AUTHORITATIVE_COMPLETION: &str = "single-authoritative-completion";
const COMPLETION_REQUIRES_CURRENT_LEASE: &str = "completion-requires-current-lease";

/// The local rules of one reference service.
pub const fn reference_rules(kind: ReferenceSystemKind) -> &'static [&'static str] {
    match kind {
        ReferenceSystemKind::TransactionalKeyValue => &[TRANSACTION_VERSION_MONOTONIC, CONFLICT_DOES_NOT_MUTATE],
        ReferenceSystemKind::ReplicatedLog => &[LOG_OFFSETS_CONTIGUOUS, RETENTION_FOLLOWS_REPLICATION],
        ReferenceSystemKind::DistributedScheduler => {
            &[SINGLE_AUTHORITATIVE_COMPLETION, COMPLETION_REQUIRES_CURRENT_LEASE]
        }
    }
}

fn kind_of_service(service: &str) -> Option<ReferenceSystemKind> {
    [
        ReferenceSystemKind::TransactionalKeyValue,
        ReferenceSystemKind::ReplicatedLog,
        ReferenceSystemKind::DistributedScheduler,
    ]
    .into_iter()
    .find(|kind| kind.as_str() == service)
}

/// How a reference operation's decision classifies the transition.
pub const fn reference_transition_class(
    operation: &ReferenceServiceOperation,
    decision: ReferenceTransitionDecision,
) -> TransitionClass {
    let is_recovery = matches!(
        operation,
        ReferenceServiceOperation::TransactionalKeyValue(TransactionalKeyValueOperation::Recover { .. })
            | ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Recover { .. })
    );
    match decision {
        _ if is_recovery => TransitionClass::Recovered,
        ReferenceTransitionDecision::Applied => TransitionClass::Applied,
        ReferenceTransitionDecision::Conflict => TransitionClass::Conflict,
        ReferenceTransitionDecision::Denied => TransitionClass::Denied,
    }
}

/// The local rules the step from `previous` to `next` violates.
// r[impl molten.fabric_simulation.local_protocol_guards]
// r[impl molten.fabric_simulation.protocol_safety]
pub fn reference_local_violations(
    previous: Option<&ReferenceServiceState>,
    next: &ReferenceServiceState,
    class: TransitionClass,
) -> Vec<&'static str> {
    let mut violations = Vec::with_capacity(2);
    let is_unchanged_required = matches!(class, TransitionClass::Conflict | TransitionClass::Denied);
    if is_unchanged_required && previous.is_some_and(|previous| previous != next) {
        violations.push(CONFLICT_DOES_NOT_MUTATE);
    }
    match (previous, next) {
        (_, ReferenceServiceState::TransactionalKeyValue(next)) => {
            let previous_version = match previous {
                Some(ReferenceServiceState::TransactionalKeyValue(previous)) => previous.version,
                _ => next.version,
            };
            if class != TransitionClass::Recovered && next.version < previous_version {
                violations.push(TRANSACTION_VERSION_MONOTONIC);
            }
        }
        (_, ReferenceServiceState::ReplicatedLog(next)) => {
            if !is_log_contiguous(next) {
                violations.push(LOG_OFFSETS_CONTIGUOUS);
            }
            let is_retention_replicated = next.retained_from == 0
                || next.replicated_through.is_some_and(|replicated| replicated >= next.retained_from);
            if !is_retention_replicated {
                violations.push(RETENTION_FOLLOWS_REPLICATION);
            }
        }
        (_, ReferenceServiceState::DistributedScheduler(next)) => {
            if class != TransitionClass::Recovered
                && let Some(ReferenceServiceState::DistributedScheduler(previous)) = previous
            {
                let is_completion_kept = previous.jobs.iter().all(|(job_id, job)| {
                    job.authoritative_completion_ref.is_none()
                        || next
                            .jobs
                            .get(job_id)
                            .is_some_and(|kept| kept.authoritative_completion_ref == job.authoritative_completion_ref)
                });
                if !is_completion_kept {
                    violations.push(SINGLE_AUTHORITATIVE_COMPLETION);
                }
            }
            let is_lease_current = next.jobs.values().all(|job| {
                let is_completed = job.phase == ReferenceJobPhase::Completed;
                is_completed == job.authoritative_completion_ref.is_some()
                    && (!is_completed || (job.lease_owner.is_some() && job.lease_epoch >= 1))
            });
            if !is_lease_current {
                violations.push(COMPLETION_REQUIRES_CURRENT_LEASE);
            }
        }
    }
    violations
}

fn is_log_contiguous(state: &ReplicatedLogState) -> bool {
    let (Some(first), Some(last)) = (state.entries.first(), state.entries.last()) else {
        return state.next_offset == state.retained_from;
    };
    first.offset == state.retained_from
        && last.offset.checked_add(1) == Some(state.next_offset)
        && state.entries.windows(2).all(|pair| pair[0].offset.checked_add(1) == Some(pair[1].offset))
}

/// The independent oracle semantics of the reference services.
///
/// It decodes projected values and applies the shared local rules; a value
/// that does not decode violates every rule of its service.
// r[impl molten.fabric_simulation.oracle_independence]
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceProtocolSemantics;

impl ProtocolSemantics for ReferenceProtocolSemantics {
    fn local_rules(&self, service: &str) -> &'static [&'static str] {
        kind_of_service(service).map_or(&[], reference_rules)
    }

    fn check_local(&self, previous: Option<&ProtocolProjection>, next: &ProtocolProjection) -> Vec<&'static str> {
        let all = self.local_rules(&next.service).to_vec();
        let Some(next_state) = decode_reference_state(&next.state) else {
            return all;
        };
        let previous_state = match previous {
            None => None,
            Some(previous) => match decode_reference_state(&previous.state) {
                Some(state) => Some(state),
                None => return all,
            },
        };
        if next_state.kind().as_str() != next.service {
            return all;
        }
        reference_local_violations(previous_state.as_ref(), &next_state, next.transition_class)
    }
}

/// A local guard violation: a typed invariant result for the shell's
/// quarantine, denial, or fail-stop policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceGuardViolation {
    pub service: ReferenceSystemKind,
    pub rule: &'static str,
}

/// Guard one executor step with the shared local rules.
// r[impl molten.fabric_simulation.local_protocol_guards]
pub fn guard_reference_transition(
    previous: &ReferenceServiceState,
    operation: &ReferenceServiceOperation,
    transition: &ReferenceServiceTransition,
) -> Result<(), ReferenceGuardViolation> {
    let class = reference_transition_class(operation, transition.decision);
    match reference_local_violations(Some(previous), &transition.next, class).first() {
        None => Ok(()),
        Some(rule) => Err(ReferenceGuardViolation {
            service: transition.next.kind(),
            rule,
        }),
    }
}
