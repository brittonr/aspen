
fn workload_choice_id(sequence: u64) -> String {
    format!("workload-{sequence:0width$}", width = REFERENCE_CHOICE_ID_WIDTH)
}

fn reference_node_id(kind: crate::fabric::ReferenceSystemKind) -> String {
    format!("node-{}", kind.as_str())
}

fn reference_invariants() -> Vec<SimulationInvariant> {
    let mut invariants =
        REQUIRED_UNIVERSAL_INVARIANTS.into_iter().map(SimulationInvariant::Universal).collect::<Vec<_>>();
    invariants.extend([
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "transaction-version-monotonic".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::TransactionalKeyValue,
            invariant_id: "conflict-does-not-mutate".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::ReplicatedLog,
            invariant_id: "log-offsets-contiguous".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::ReplicatedLog,
            invariant_id: "retention-follows-replication".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::DistributedScheduler,
            invariant_id: "single-authoritative-completion".to_string(),
        },
        SimulationInvariant::ExtensionSemantic {
            service: crate::fabric::ReferenceSystemKind::DistributedScheduler,
            invariant_id: "completion-requires-current-lease".to_string(),
        },
    ]);
    invariants
}
