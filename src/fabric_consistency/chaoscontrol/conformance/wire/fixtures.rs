//! Supplied offline committed-apply fixtures, not guest execution evidence.

pub(super) fn id(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("canonical fixture identity")
}

pub(super) fn profiles() -> (chaoscontrol_smr::observation::ProtocolProfile, chaoscontrol_smr::SmrWorkloadPlan) {
    let plan = chaoscontrol_smr::admit_profile(&chaoscontrol_smr::SmrWorkloadProfile {
        schema: chaoscontrol_smr::PROFILE_SCHEMA.into(),
        profile_id: "aspen-wire-offline-control".into(),
        initial_state_ref: id("initial-state"),
        observation_mode: chaoscontrol_smr::ObservationMode::Lossless,
        bounds: chaoscontrol_smr::WorkloadBounds {
            maximum_commands: 8,
            maximum_command_bytes: 32,
            maximum_clients: 2,
            maximum_concurrency: 2,
            maximum_virtual_progress: 100,
            maximum_trace_events: 16,
            maximum_fault_actions: 8,
            maximum_replay_events: 16,
            maximum_evidence_bytes: 131_072,
            maximum_reduction_attempts: 8,
        },
        liveness: chaoscontrol_smr::LivenessProfile {
            profile_id: "wire-progress-control".into(),
            required_progress: 1,
            virtual_progress_horizon: 100,
        },
    })
    .expect("producer-admitted workload");
    let protocol = chaoscontrol_smr::observation::ProtocolProfile {
        schema: chaoscontrol_smr::observation::PROFILE_SCHEMA.into(),
        protocol_ref: id("molten-consensus-offline-control"),
        projection_schema_ref: id("chain-observation-schema"),
        boundary_schema_ref: id("committed-apply-boundary-schema"),
        oracle_adapter_ref: id("molten-observer"),
        independent_oracle_ref: id("independent-offline-control-oracle"),
        novelty_selection_ref: id("offline-control-selection"),
        participants: (0..2)
            .map(|guest_id| chaoscontrol_smr::observation::Participant {
                participant_ref: id(&format!("replica-{guest_id}")),
                producer_ref: id(&format!("observer-source-{guest_id}")),
                guest_id,
                generation: 7,
            })
            .collect(),
        bounds: chaoscontrol_smr::observation::Bounds {
            records: 16,
            record_bytes: 65_536,
            total_bytes: 131_072,
            boundaries: 8,
            cohort_backlog: 8,
            oracle_work: 16,
        },
        marker_policy: chaoscontrol_smr::observation::MarkerPolicy::Disabled,
        non_claims: chaoscontrol_smr::observation::NON_CLAIMS.iter().map(|value| (*value).into()).collect(),
    };
    protocol.admit().expect("producer-admitted protocol");
    (protocol, plan)
}

pub(super) fn context(index: u64) -> super::ProjectionContext {
    super::ProjectionContext {
        group_ref: id("group"),
        source_sequence: index,
        boundary_ref: id(&format!("committed-boundary-{index}")),
        scheduler_ref: id("offline-scheduler-control"),
        transition_class_ref: id("committed-apply-transition"),
    }
}

pub(super) fn projector(plan: &chaoscontrol_smr::SmrWorkloadPlan) -> super::super::super::ChaosControlChainProjector {
    super::super::super::ChaosControlChainProjector::bind(
        super::super::super::ChaosControlConformanceProfile {
            chaoscontrol_profile_ref: plan.profile_ref.clone(),
            observation_mode: super::super::super::ChaosControlObservationMode::Lossless,
            max_command_bytes: plan.profile.bounds.maximum_command_bytes,
            max_projected_observations: plan.profile.bounds.maximum_trace_events,
        },
        &plan.profile.initial_state_ref,
    )
    .expect("existing Molten projection profile")
}

pub(super) fn apply(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    participant: usize,
    index: u64,
    bytes: &[u8],
) -> super::super::super::CommittedApplyObservation {
    super::super::super::CommittedApplyObservation {
        group_ref: id("group"),
        replica_ref: protocol.participants[participant].participant_ref.clone(),
        command_index: index,
        operation_ref: id(&format!("operation-{index}")),
        command_ref: id(&format!("command-{index}")),
        command_bytes: bytes.to_vec(),
        application_state_ref: id(&format!("application-state-{index}")),
        lifecycle_generation: protocol.participants[participant].generation,
        application_receipt_ref: id(&format!("application-receipt-{participant}-{index}")),
    }
}

pub(super) fn observed(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
) -> super::super::super::ChaosControlChainObservation {
    projector(plan).project(&apply(protocol, 0, 1, b"first-command")).expect("supplied committed apply")
}

pub(super) fn bound(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
) -> super::BoundObservation {
    super::project(protocol, plan, &context(1), &observed(protocol, plan)).expect("typed encoder")
}
