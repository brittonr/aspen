//! Shared fixtures: the named model profiles, operation identities, and admission attempts.

pub(super) const INITIAL_VIEW: u64 = 1;
pub(super) const NEXT_VIEW: u64 = 2;
pub(super) const THREE_NODES: usize = 3;
pub(super) const FIVE_NODES: usize = 5;
pub(super) const THREE_NODE_SUPERQUORUM: usize = 3;
pub(super) const FIVE_NODE_SUPERQUORUM: usize = 4;
/// The order-preserving active proposer of every attempt fixture; an admitted replica.
pub(super) const PROPOSER: &str = "node-0";
const SESSION_SEQUENCE: u64 = 7;
const EXTENSION_GENERATION: u64 = 3;
const ENGINE_EPOCH: u64 = 4;
const MODEL_COMMAND_BOUND: usize = 8;
const MODEL_KEY_BOUND: usize = 4;
const MODEL_VIEW_BOUND: usize = 4;
const MODEL_STEP_BOUND: usize = 64;

pub(super) fn strings(values: &[&str]) -> alloc::collections::BTreeSet<alloc::string::String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn replica(index: usize) -> alloc::string::String {
    alloc::format!("node-{index}")
}

pub(super) fn quorums(node_count: usize) -> crate::envelope::DerivedQuorums {
    crate::envelope::derive_quorums(u32::try_from(node_count).expect("bounded node count")).expect("named quorum")
}

pub(super) fn profile(node_count: usize) -> crate::envelope::FastPathModelProfile {
    crate::envelope::FastPathModelProfile {
        profile_id: if node_count == THREE_NODES {
            crate::envelope::THREE_REPLICA_PROFILE.to_owned()
        } else {
            crate::envelope::FIVE_REPLICA_PROFILE.to_owned()
        },
        source: crate::envelope::SourceCohort {
            paper: crate::envelope::JETPACK_PAPER.to_owned(),
            artifact_source: crate::envelope::JETPACK_ARTIFACT_SOURCE.to_owned(),
            artifact_revision: crate::envelope::JETPACK_ARTIFACT_REVISION.to_owned(),
            artifact_license: "MIT".to_owned(),
        },
        base_model_ref: "blake3:base-model".to_owned(),
        conflict_contract_ref: "blake3:conflict-contract".to_owned(),
        fault_model: crate::envelope::CRASH_FAULT_MODEL.to_owned(),
        node_count,
        active_proposers: strings(&[PROPOSER]),
        max_commands: MODEL_COMMAND_BOUND,
        max_keys: MODEL_KEY_BOUND,
        max_views: MODEL_VIEW_BOUND,
        max_steps: MODEL_STEP_BOUND,
        base_ordering: crate::envelope::BaseOrderingPrerequisites {
            receive_order_preserved: true,
            proposal_order_preserved: true,
            execution_order_preserved: true,
            acknowledgement_waits_for_proposal_evidence: false,
        },
        selection: crate::envelope::SelectionMode::PureModel,
        claim_profile: crate::envelope::MODEL_ONLY_CLAIM.to_owned(),
        invariant_names: strings(crate::envelope::required_invariants()),
        non_claims: strings(crate::envelope::required_non_claims()),
    }
}

pub(super) fn operation(command: &str) -> crate::quorate::OperationIdentity {
    crate::quorate::OperationIdentity {
        command_ref: command.to_owned(),
        session_ref: "session-a".to_owned(),
        session_sequence: SESSION_SEQUENCE,
        group_ref: "group-a".to_owned(),
        extension_generation: EXTENSION_GENERATION,
        application_schema_ref: "schema-a".to_owned(),
        policy_ref: "policy-a".to_owned(),
        authority_ref: "authority-a".to_owned(),
        resource_ref: "resource-a".to_owned(),
        engine_epoch: ENGINE_EPOCH,
    }
}

pub(super) fn acknowledgement(
    replica_id: alloc::string::String,
    operation: &crate::quorate::OperationIdentity,
) -> crate::quorate::FastAcknowledgement {
    crate::quorate::FastAcknowledgement {
        replica_id,
        acceleration_view: INITIAL_VIEW,
        base_view: INITIAL_VIEW,
        operation: operation.clone(),
    }
}

/// Every admitted replica, the active proposer included, acknowledges in the attempt's view.
pub(super) fn attempt(node_count: usize) -> crate::quorate::StableViewAttempt {
    let operation = operation("command-a");
    crate::quorate::StableViewAttempt {
        acknowledgements: (0..node_count).map(|index| acknowledgement(replica(index), &operation)).collect(),
        operation: operation.clone(),
        original_operation: operation,
        acceleration_view: INITIAL_VIEW,
        base_view: INITIAL_VIEW,
        conflict_free: true,
        replicas: (0..node_count).map(replica).collect(),
        promises: alloc::vec![crate::quorate::ProposerPromise {
            proposer_id: PROPOSER.to_owned(),
            acceleration_view: INITIAL_VIEW,
            base_view: INITIAL_VIEW,
            proposal_order_preserved: true,
        }],
        active_proposers: strings(&[PROPOSER]),
        original_path_available: true,
    }
}
