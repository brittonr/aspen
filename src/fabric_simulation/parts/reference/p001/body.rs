
pub fn default_reference_operations() -> Vec<(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)> {
    let kv_request = blake3_ref(b"reference-kv-commit");
    let log_append_request = blake3_ref(b"reference-log-append");
    let log_replicate_request = blake3_ref(b"reference-log-replicate");
    let scheduler_submit_request = blake3_ref(b"reference-scheduler-submit");
    let scheduler_lease_request = blake3_ref(b"reference-scheduler-lease");
    let scheduler_complete_request = blake3_ref(b"reference-scheduler-complete");
    vec![
        (
            crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            kv_request,
            ReferenceServiceOperation::TransactionalKeyValue(TransactionalKeyValueOperation::Commit {
                expected_version: initial_transaction_version(),
                writes: vec![("key-a".to_string(), blake3_ref(b"value-a"))],
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::ReplicatedLog,
            log_append_request,
            ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Append {
                payload_ref: blake3_ref(b"log-entry-a"),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::ReplicatedLog,
            log_replicate_request,
            ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::ReplicateThrough { offset: 0 }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_submit_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Submit {
                job_id: "job-a".to_string(),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_lease_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Lease {
                job_id: "job-a".to_string(),
                owner: "worker-a".to_string(),
            }),
        ),
        (
            crate::fabric::ReferenceSystemKind::DistributedScheduler,
            scheduler_complete_request,
            ReferenceServiceOperation::DistributedScheduler(DistributedSchedulerOperation::Complete {
                job_id: "job-a".to_string(),
                owner: "worker-a".to_string(),
                completion_ref: blake3_ref(b"job-a-completion"),
            }),
        ),
    ]
}

pub fn operations_for_kind(
    operations: &[(crate::fabric::ReferenceSystemKind, String, ReferenceServiceOperation)],
    kind: crate::fabric::ReferenceSystemKind,
) -> std::collections::BTreeMap<String, ReferenceServiceOperation> {
    operations
        .iter()
        .filter(|(operation_kind, _, _)| *operation_kind == kind)
        .map(|(_, request_ref, operation)| (request_ref.clone(), operation.clone()))
        .collect()
}

pub fn simulation_port_id(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}", class.as_str())
}

pub fn simulation_command_schema(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}.command.v1", class.as_str())
}

pub fn simulation_event_schema(class: crate::fabric::FabricPortClass) -> String {
    format!("molten.fabric.simulation.{}.event.v1", class.as_str())
}

pub fn blake3_ref(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn authority_for_class(class: crate::fabric::FabricPortClass) -> crate::fabric::FabricAuthority {
    match class {
        crate::fabric::FabricPortClass::Authority => crate::fabric::FabricAuthority::ProtocolOwnership,
        crate::fabric::FabricPortClass::Transport => crate::fabric::FabricAuthority::Transport,
        crate::fabric::FabricPortClass::DurableState => crate::fabric::FabricAuthority::DurableState,
        crate::fabric::FabricPortClass::Execution => crate::fabric::FabricAuthority::Execution,
        crate::fabric::FabricPortClass::Time => crate::fabric::FabricAuthority::Time,
        crate::fabric::FabricPortClass::Scheduling => crate::fabric::FabricAuthority::Scheduling,
        crate::fabric::FabricPortClass::Membership => crate::fabric::FabricAuthority::Membership,
        crate::fabric::FabricPortClass::Placement => crate::fabric::FabricAuthority::Placement,
        crate::fabric::FabricPortClass::Consistency => crate::fabric::FabricAuthority::Consistency,
        crate::fabric::FabricPortClass::Supervision => crate::fabric::FabricAuthority::Supervision,
        crate::fabric::FabricPortClass::Policy => crate::fabric::FabricAuthority::Policy,
        crate::fabric::FabricPortClass::Resources => crate::fabric::FabricAuthority::Resources,
        crate::fabric::FabricPortClass::Simulation => crate::fabric::FabricAuthority::Simulation,
        crate::fabric::FabricPortClass::Evidence => crate::fabric::FabricAuthority::Evidence,
    }
}

fn resource_for_class(class: crate::fabric::FabricPortClass) -> crate::fabric::FabricResource {
    match class {
        crate::fabric::FabricPortClass::Transport => crate::fabric::FabricResource::NetworkBytes,
        crate::fabric::FabricPortClass::DurableState => crate::fabric::FabricResource::StorageBytes,
        crate::fabric::FabricPortClass::Execution => crate::fabric::FabricResource::ExecutionMillis,
        crate::fabric::FabricPortClass::Time | crate::fabric::FabricPortClass::Scheduling => {
            crate::fabric::FabricResource::LogicalTime
        }
        crate::fabric::FabricPortClass::Evidence => crate::fabric::FabricResource::Diagnostics,
        crate::fabric::FabricPortClass::Authority
        | crate::fabric::FabricPortClass::Membership
        | crate::fabric::FabricPortClass::Placement
        | crate::fabric::FabricPortClass::Consistency
        | crate::fabric::FabricPortClass::Supervision
        | crate::fabric::FabricPortClass::Policy
        | crate::fabric::FabricPortClass::Resources
        | crate::fabric::FabricPortClass::Simulation => crate::fabric::FabricResource::Memory,
    }
}

fn declared_faults(class: crate::fabric::FabricPortClass) -> Vec<SimulationFaultKind> {
    match class {
        crate::fabric::FabricPortClass::Transport => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::Drop,
            SimulationFaultKind::Duplicate,
            SimulationFaultKind::Reorder,
            SimulationFaultKind::Partition,
            SimulationFaultKind::Reset,
        ],
        crate::fabric::FabricPortClass::DurableState => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::BoundedCorruption,
            SimulationFaultKind::CapacityExhaustion,
            SimulationFaultKind::Crash,
        ],
        crate::fabric::FabricPortClass::Execution => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::CapacityExhaustion,
            SimulationFaultKind::Pause,
            SimulationFaultKind::Crash,
        ],
        crate::fabric::FabricPortClass::Time | crate::fabric::FabricPortClass::Scheduling => vec![
            SimulationFaultKind::Delay,
            SimulationFaultKind::ClockSkew,
            SimulationFaultKind::ClockJump,
            SimulationFaultKind::Pause,
        ],
        crate::fabric::FabricPortClass::Membership => {
            vec![SimulationFaultKind::MembershipChange, SimulationFaultKind::Partition]
        }
        crate::fabric::FabricPortClass::Placement => vec![SimulationFaultKind::PlacementReplacement],
        crate::fabric::FabricPortClass::Consistency => vec![
            SimulationFaultKind::ConsistencyQuorumLoss,
            SimulationFaultKind::Partition,
        ],
        crate::fabric::FabricPortClass::Authority | crate::fabric::FabricPortClass::Policy => {
            vec![SimulationFaultKind::AuthorityRevocation]
        }
        crate::fabric::FabricPortClass::Supervision => vec![
            SimulationFaultKind::Pause,
            SimulationFaultKind::Crash,
            SimulationFaultKind::Restart,
        ],
        crate::fabric::FabricPortClass::Resources => vec![SimulationFaultKind::CapacityExhaustion],
        crate::fabric::FabricPortClass::Simulation | crate::fabric::FabricPortClass::Evidence => {
            vec![SimulationFaultKind::Delay]
        }
    }
}
