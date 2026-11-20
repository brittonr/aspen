//! Shared bounded fixtures for the conformance fault and safety tests.

pub(super) fn id(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("canonical fixture identity")
}

pub(super) fn plan() -> chaoscontrol_smr::SmrWorkloadPlan {
    chaoscontrol_smr::admit_profile(&chaoscontrol_smr::SmrWorkloadProfile {
        schema: chaoscontrol_smr::PROFILE_SCHEMA.into(),
        profile_id: "molten-fault-matrix-control".into(),
        initial_state_ref: id("fault-matrix-initial-state"),
        observation_mode: chaoscontrol_smr::ObservationMode::Lossless,
        bounds: chaoscontrol_smr::WorkloadBounds {
            maximum_commands: 8,
            maximum_command_bytes: 32,
            maximum_clients: 3,
            maximum_concurrency: 3,
            maximum_virtual_progress: 64,
            maximum_trace_events: 64,
            maximum_fault_actions: 8,
            maximum_replay_events: 64,
            maximum_evidence_bytes: 131_072,
            maximum_reduction_attempts: 8,
        },
        liveness: chaoscontrol_smr::LivenessProfile {
            profile_id: "molten-progress-control".into(),
            required_progress: 1,
            virtual_progress_horizon: 64,
        },
    })
    .expect("producer-admitted workload")
}

pub(super) fn digest(plan: &chaoscontrol_smr::SmrWorkloadPlan, index: u64, prior: &str, command: &[u8]) -> String {
    chaoscontrol_smr::chain_transition(&plan.profile_ref, index, prior, command).expect("chain transition")
}

pub(super) fn applied(
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    participant: &str,
    index: u64,
    prior: &str,
    command: &[u8],
    operation: &str,
    receipt: &str,
) -> super::safety::AppliedChain {
    applied_at(plan, participant, index, index, prior, command, operation, receipt)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn applied_at(
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    participant: &str,
    index: u64,
    event_sequence: u64,
    prior: &str,
    command: &[u8],
    operation: &str,
    receipt: &str,
) -> super::safety::AppliedChain {
    let digest = digest(plan, index, prior, command);
    super::safety::AppliedChain {
        operation_ref: operation.into(),
        participant_ref: participant.into(),
        command_ref: id(&format!("{operation}-command")),
        application_receipt_ref: receipt.into(),
        chain: chaoscontrol_smr::ChainObservation {
            event_sequence,
            profile_ref: plan.profile_ref.clone(),
            replica_id: participant.into(),
            command_index: index,
            prior_digest: prior.into(),
            digest: digest.clone(),
            application_state_ref: digest,
        },
    }
}
