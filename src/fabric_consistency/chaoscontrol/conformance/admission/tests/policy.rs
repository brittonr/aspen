fn workload() -> chaoscontrol_smr::SmrWorkloadProfile {
    chaoscontrol_smr::SmrWorkloadProfile {
        schema: chaoscontrol_smr::PROFILE_SCHEMA.into(),
        profile_id: "consumer-instrumentation-control".into(),
        initial_state_ref: id("initial-state"),
        observation_mode: chaoscontrol_smr::ObservationMode::Lossless,
        bounds: chaoscontrol_smr::WorkloadBounds {
            maximum_commands: 4,
            maximum_command_bytes: 1_024,
            maximum_clients: 2,
            maximum_concurrency: 2,
            maximum_virtual_progress: 20,
            maximum_trace_events: 32,
            maximum_fault_actions: 4,
            maximum_replay_events: 128,
            maximum_evidence_bytes: 1_048_576,
            maximum_reduction_attempts: 4,
        },
        liveness: chaoscontrol_smr::LivenessProfile {
            profile_id: "bounded-stabilization".into(),
            required_progress: 2,
            virtual_progress_horizon: 10,
        },
    }
}

fn protocol(adapter_ref: &str) -> chaoscontrol_smr::observation::ProtocolProfile {
    chaoscontrol_smr::observation::ProtocolProfile {
        schema: chaoscontrol_smr::observation::PROFILE_SCHEMA.into(),
        protocol_ref: id("protocol"),
        projection_schema_ref: id("projection-schema"),
        boundary_schema_ref: id("boundary-schema"),
        oracle_adapter_ref: adapter_ref.to_owned(),
        independent_oracle_ref: id("independent-corpus-oracle"),
        novelty_selection_ref: id("novelty"),
        participants: (1..=2)
            .map(|guest_id| chaoscontrol_smr::observation::Participant {
                participant_ref: id(&format!("participant-{guest_id}")),
                producer_ref: id(&format!("producer-{guest_id}")),
                guest_id,
                generation: 7,
            })
            .collect(),
        bounds: chaoscontrol_smr::observation::Bounds {
            records: 32,
            record_bytes: 8_192,
            total_bytes: 262_144,
            boundaries: 8,
            cohort_backlog: 8,
            oracle_work: 32,
        },
        marker_policy: chaoscontrol_smr::observation::MarkerPolicy::Disabled,
        non_claims: chaoscontrol_smr::observation::NON_CLAIMS.iter().map(|s| (*s).into()).collect(),
    }
}

fn operations() -> Vec<super::ExpectedOperation> {
    (1..=2)
        .map(|index| {
            let command =
                crate::preserves_rail::record("consumer-control-command", vec![crate::preserves_rail::string(
                    index.to_string(),
                )]);
            super::ExpectedOperation {
                client_session: "control-client".into(),
                sequence: index,
                operation_ref: super::operation_ref("control-client", index).unwrap(),
                command_index: index,
                command_ref: crate::preserves_rail::canonical_hash(&command).unwrap(),
                command_bytes: crate::codec::canonical_bytes(&command).unwrap(),
                application_state_ref: id(&format!("state-{index}")),
                boundary_ref: id(&format!("boundary-{index}")),
                transition_class_ref: id("committed-apply"),
            }
        })
        .collect()
}

fn expected() -> super::ExpectedCohort {
    let workload = workload();
    let adapter_ref = id("adapter");
    let protocol = protocol(&adapter_ref);
    let operations = operations();
    let source_ref = id("molten-source");
    let build_ref = id("molten-build");
    let sources = (1..=2)
        .map(|guest_id| chaoscontrol_smr::observation::guest::GuestSource {
            guest_id,
            executable_ref: build_ref.clone(),
            source_revision_ref: source_ref.clone(),
            adapter_ref: adapter_ref.clone(),
            launch_contract_ref: id("instrumentation-not-a-launch"),
        })
        .collect();
    super::ExpectedCohort {
        schema: super::EXPECTED_SCHEMA.into(),
        producer_revision: super::super::PRODUCER_REVISION.into(),
        workload,
        protocol,
        group_ref: id("group"),
        boundary_ref: operations.last().unwrap().boundary_ref.clone(),
        scheduler_ref: id("schedule"),
        molten_source_ref: source_ref.clone(),
        molten_build_ref: build_ref.clone(),
        dependency_closure_ref: id("closure"),
        adapter_ref,
        observer_ref: id("observer"),
        sources,
        evidence_class: chaoscontrol_smr::observation::guest::EvidenceClass::Instrumentation,
        claim: "bounded-instrumentation".into(),
        operations,
        faults: Vec::new(),
    }
}
