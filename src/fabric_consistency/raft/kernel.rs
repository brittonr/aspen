//! The shell's binding of the `molten-raft-core` kernel.
//!
//! The kernel decides; this module supplies the canonical identity port (BLAKE3 over canonical
//! packed Preserves records) and converts kernel refusals into the shell's `Failure`, keeping
//! each refusal's class: invariant denials stay `InvalidHarness` with their text, contradicting
//! recovery evidence stays a classified `MalformedObservation`, and identity encoding failures
//! stay `Preserves`.

use super::*;

/// The canonical identity port every live replica, durable store, and peer agrees on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CanonicalReplicaIdentity;

impl molten_raft_core::identity::ReplicaIdentity for CanonicalReplicaIdentity {
    fn election_timer_ref(
        &self,
        timer: &molten_raft_core::identity::ElectionTimer<'_>,
    ) -> std::result::Result<String, molten_raft_core::denial::IdentityError> {
        election_timer_ref(timer.group_binding_ref, timer.node_id, timer.service_generation, timer.term, timer.sequence)
            .map_err(identity_error)
    }

    fn snapshot_ref(
        &self,
        snapshot: &ReplicaSnapshot,
    ) -> std::result::Result<String, molten_raft_core::denial::IdentityError> {
        snapshot_ref(snapshot).map_err(identity_error)
    }

    fn recovery_barrier_ref(
        &self,
        timer_ref: &str,
        term: u64,
    ) -> std::result::Result<String, molten_raft_core::denial::IdentityError> {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-recovery-barrier-v1", vec![
            crate::preserves_rail::string(timer_ref),
            crate::preserves_rail::u64_value(term),
        ]))
        .map_err(identity_error)
    }

    fn entry_ref(
        &self,
        entry: &ReplicatedEntry,
    ) -> std::result::Result<String, molten_raft_core::denial::IdentityError> {
        canonical::entry_identity_ref(entry).map_err(identity_error)
    }

    fn recovery_corpus_ref(
        &self,
        corpus: &molten_raft_core::identity::RecoveryCorpus<'_>,
    ) -> std::result::Result<String, molten_raft_core::denial::IdentityError> {
        canonical::recovery_corpus_ref(corpus.term, corpus.commit_index, corpus.log, corpus.snapshot)
            .map_err(identity_error)
    }
}

fn identity_error(failure: crate::error::Failure) -> molten_raft_core::denial::IdentityError {
    match failure {
        crate::error::Failure::Preserves(detail) => molten_raft_core::denial::IdentityError::new(detail),
        other => molten_raft_core::denial::IdentityError::new(other.to_string()),
    }
}

/// Convert a kernel refusal into the shell's `Failure` without losing its class or text.
pub fn replica_failure(error: molten_raft_core::denial::ReplicaError) -> crate::error::Failure {
    match error {
        molten_raft_core::denial::ReplicaError::ContentRef {
            label: Some(label),
            issue,
        } => crate::error::Failure::invalid_harness(format!(
            "invalid {label}: {}",
            crate::error::Failure::invalid_harness(issue.to_string())
        )),
        molten_raft_core::denial::ReplicaError::Identity(error) => {
            crate::error::Failure::Preserves(error.detail().to_owned())
        }
        error => match error.class() {
            molten_raft_core::denial::ReplicaErrorClass::Invalid => {
                crate::error::Failure::invalid_harness(error.to_string())
            }
            molten_raft_core::denial::ReplicaErrorClass::MalformedObservation => {
                crate::error::Failure::classified(crate::error::FailureClass::MalformedObservation, error.to_string())
            }
            molten_raft_core::denial::ReplicaErrorClass::Encoding => {
                crate::error::Failure::Preserves(error.to_string())
            }
        },
    }
}

// r[impl molten.fabric_consistency.live_raft]
// r[impl molten.fabric_consistency.group_isolation]
pub fn apply_replica_event(state: &ReplicaState, event: ReplicaEvent) -> crate::error::Result<ReplicaTransition> {
    molten_raft_core::transition::apply_replica_event(state, event, &CanonicalReplicaIdentity).map_err(replica_failure)
}

/// The snapshot threshold must leave room below the log bound for entries that are not yet applied,
/// and the duplicate-request window must be positive and no wider than the largest admitted log.
// r[impl molten.consensus.snapshot_threshold]
// r[impl molten.consensus.bounded_request_dedup]
pub fn validate_retention_bounds(profile: &ReplicaProfile) -> crate::error::Result<()> {
    molten_raft_core::admission::validate_retention_bounds(profile).map_err(replica_failure)
}

pub(crate) fn validate_recovered_replica_state(state: &ReplicaState) -> crate::error::Result<()> {
    molten_raft_core::transition::validate_recovered_replica_state(state, &CanonicalReplicaIdentity)
        .map_err(replica_failure)
}

/// Shape validation every envelope passes before it is encoded for or decoded from the wire.
pub(crate) fn validate_wire_envelope(envelope: &ReplicaMessageEnvelope) -> crate::error::Result<()> {
    molten_raft_core::transition::wire::validate_envelope(envelope, &CanonicalReplicaIdentity).map_err(replica_failure)
}

pub(crate) fn snapshot_ref(snapshot: &ReplicaSnapshot) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-replica-snapshot-identity-v1", vec![
        crate::preserves_rail::string(&snapshot.group_binding_ref),
        crate::preserves_rail::string(&snapshot.membership_ref),
        crate::preserves_rail::u64_value(snapshot.config_epoch),
        crate::preserves_rail::u64_value(snapshot.fencing_epoch),
        crate::preserves_rail::u64_value(snapshot.last_included_index),
        crate::preserves_rail::u64_value(snapshot.last_included_term),
        crate::preserves_rail::string(&snapshot.application_state_ref),
        crate::preserves_rail::sequence(
            snapshot
                .completed_requests
                .iter()
                .map(|(request_ref, index)| {
                    crate::preserves_rail::record("completed-request", vec![
                        crate::preserves_rail::string(request_ref),
                        crate::preserves_rail::u64_value(*index),
                    ])
                })
                .collect(),
        ),
    ]))
}

pub(crate) fn election_timer_ref(
    group_binding_ref: &str,
    node_id: &str,
    service_generation: u64,
    term: u64,
    sequence: u64,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-election-timer-v1", vec![
        crate::preserves_rail::string(group_binding_ref),
        crate::preserves_rail::string(node_id),
        crate::preserves_rail::u64_value(service_generation),
        crate::preserves_rail::u64_value(term),
        crate::preserves_rail::u64_value(sequence),
    ]))
}
