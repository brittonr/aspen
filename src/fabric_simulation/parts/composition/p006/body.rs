fn reference_execution_differential(
    prepared: &PreparedReferenceWorld,
    events: &[PortEvent],
) -> crate::error::Result<ExecutionDifferential> {
    let simulation_side_ref = blake3_ref(FABRIC_SIMULATION_PROFILE_ID.as_bytes());
    let live_side_ref = blake3_ref(b"reviewed-live-port-contract-profile-v1");
    let steps = prepared
        .world
        .admitted
        .manifest
        .workload
        .iter()
        .map(|step| LiveWorkloadStep {
            sequence: step.sequence,
            class: match step.service {
                crate::fabric::ReferenceSystemKind::TransactionalKeyValue | crate::fabric::ReferenceSystemKind::ReplicatedLog => {
                    crate::fabric::FabricPortClass::DurableState
                }
                crate::fabric::ReferenceSystemKind::DistributedScheduler => crate::fabric::FabricPortClass::Scheduling,
            },
            request_ref: step.request_ref.clone(),
            permitted: if step.expected_failure_class.is_some() {
                vec![OutcomeClass::Rejected]
            } else {
                vec![OutcomeClass::Delivered]
            },
            ordering_key: Some(format!("node:{}", step.node_id)),
        })
        .collect::<Vec<_>>();
    let workload = LiveWorkload::new(steps)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("execution workload denied: {issues:?}")))?;
    let simulated = workload
        .steps
        .iter()
        .map(|step| {
            events.iter().find(|event| event.request_ref == step.request_ref).map(|event| {
                if event.fault.is_some() {
                    OutcomeClass::Rejected
                } else {
                    OutcomeClass::Delivered
                }
            })
        })
        .collect::<Vec<_>>();
    execution_differential(ExecutionDifferentialInput {
        workload: &workload,
        simulation_side_ref: &simulation_side_ref,
        live_side_ref: &live_side_ref,
        simulated: &simulated,
        live: None,
    })
}
