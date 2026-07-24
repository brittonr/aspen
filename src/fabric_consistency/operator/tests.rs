use super::*;
use crate::fabric_consistency::raft::ReplicaEvidenceKind;

const MAX_COMMAND_BYTES: u64 = 4_096;
const MAX_IN_FLIGHT: u32 = 8;

#[derive(Debug, Default)]
struct RecordingEffects {
    calls: usize,
}

impl ConsistencyOperatorEffects for RecordingEffects {
    fn apply(&mut self, action: ConsistencyOperatorAction, _plan: &ConsistencyPortPlan) -> Result<String> {
        self.calls += 1;
        Ok(test_ref(action.as_str()))
    }
}

// r[verify molten.fabric_consistency.operator_readback]
#[test]
fn operator_dry_run_preflights_without_effects_and_applied_inspect_calls_once() {
    let declared = declared_binding();
    let create = plan_consistency_operator_action(&declared, ConsistencyOperatorRequest {
        command: command_for(&declared, ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        }),
        dry_run: true,
    })
    .expect("create preflight");
    assert!(create.plan.admitted());
    let mut effects = RecordingEffects::default();
    let dry_run = execute_consistency_operator_action(&create, &mut effects).expect("create dry run");
    assert_eq!(dry_run.status, ConsistencyOperatorExecutionStatus::DryRun);
    assert_eq!(effects.calls, 0);

    let active = active_binding();
    let inspect = plan_consistency_operator_action(&active, ConsistencyOperatorRequest {
        command: command_for(&active, ConsistencyOperation::Status),
        dry_run: false,
    })
    .expect("inspect preflight");
    let applied = execute_consistency_operator_action(&inspect, &mut effects).expect("inspect apply");
    assert_eq!(applied.status, ConsistencyOperatorExecutionStatus::Applied);
    assert_eq!(effects.calls, 1);
    crate::preserves_rail::validate_content_ref(&applied.execution_ref).expect("operator execution ref");
}

// r[verify molten.fabric_consistency.operator_readback]
#[test]
fn operator_denial_and_unsupported_action_produce_no_effects() {
    let active = active_binding();
    let remove = plan_consistency_operator_action(&active, ConsistencyOperatorRequest {
        command: command_for(&active, ConsistencyOperation::Remove),
        dry_run: false,
    })
    .expect("remove denial preflight");
    assert!(!remove.plan.admitted());
    let mut effects = RecordingEffects::default();
    let denied = execute_consistency_operator_action(&remove, &mut effects).expect("remove denied");
    assert_eq!(denied.status, ConsistencyOperatorExecutionStatus::Denied);
    assert_eq!(effects.calls, 0);

    let unsupported = plan_consistency_operator_action(&active, ConsistencyOperatorRequest {
        command: command_for(&active, ConsistencyOperation::Health),
        dry_run: true,
    })
    .expect_err("health is outside the bounded operator action set");
    assert!(unsupported.to_string().contains("outside"));
}

// r[verify molten.fabric_consistency.operator_readback]
#[test]
fn operator_applies_drain_snapshot_recover_and_remove_through_one_effect_shell() {
    let active = active_binding();
    let mut effects = RecordingEffects::default();
    for operation in [
        ConsistencyOperation::Drain,
        ConsistencyOperation::Snapshot {
            snapshot_policy_ref: test_ref("operator-snapshot-policy"),
        },
        ConsistencyOperation::Recover {
            snapshot_ref: test_ref("operator-snapshot"),
            durable_boundary_ref: test_ref("operator-durable-boundary"),
        },
    ] {
        let preflight = plan_consistency_operator_action(&active, ConsistencyOperatorRequest {
            command: command_for(&active, operation),
            dry_run: false,
        })
        .expect("active maintenance preflight");
        let execution = execute_consistency_operator_action(&preflight, &mut effects).expect("maintenance execution");
        assert_eq!(execution.status, ConsistencyOperatorExecutionStatus::Applied);
    }
    let draining = draining_binding();
    let remove = plan_consistency_operator_action(&draining, ConsistencyOperatorRequest {
        command: command_for(&draining, ConsistencyOperation::Remove),
        dry_run: false,
    })
    .expect("remove preflight");
    let removed = execute_consistency_operator_action(&remove, &mut effects).expect("remove execution");
    assert_eq!(removed.status, ConsistencyOperatorExecutionStatus::Applied);
    assert_eq!(effects.calls, 4);
}

// r[verify molten.fabric_consistency.operator_readback]
#[test]
fn operator_readback_is_bounded_and_rejects_substituted_group_identity() {
    let active = active_binding();
    let records = (0..(MAX_OPERATOR_EVIDENCE_REFS + 1))
        .map(|offset| ReplicaEvidenceRecord {
            sequence: u64::try_from(offset + 1).expect("bounded evidence sequence"),
            kind: ReplicaEvidenceKind::Commit,
            term: INITIAL_CONSISTENCY_EPOCH,
            index: INITIAL_CONSISTENCY_EPOCH,
            source_ref: test_ref(&format!("operator-source-{offset}")),
            quorum_evidence_ref: None,
            quorum_members: Vec::new(),
            evidence_ref: test_ref(&format!("operator-evidence-{offset}")),
        })
        .collect::<Vec<_>>();
    let replica = ConsistencyOperatorReplicaState {
        group_binding_ref: active.binding_ref.clone(),
        service_generation: active.service_generation,
        node_id: "node-a".to_string(),
        role: ReplicaRole::Leader,
        lifecycle: ReplicaLifecycle::Running,
        term: INITIAL_CONSISTENCY_EPOCH,
        commit_index: INITIAL_CONSISTENCY_EPOCH,
        last_applied: INITIAL_CONSISTENCY_EPOCH,
    };
    let health = health();
    let readback = consistency_operator_readback(&active, &replica, &records, &health).expect("operator readback");
    assert_eq!(readback.selected_evidence_refs.len(), MAX_OPERATOR_EVIDENCE_REFS);
    assert!(readback.evidence_truncated);
    assert!(!readback.production_admitted);
    crate::preserves_rail::validate_content_ref(&readback.readback_ref).expect("operator readback ref");

    let mut substituted = replica;
    substituted.group_binding_ref = test_ref("substituted-group");
    let error = consistency_operator_readback(&active, &substituted, &records, &health)
        .expect_err("substituted readback group must deny");
    assert!(error.to_string().contains("binding mismatch"));
}

fn test_ref(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-consistency-operator-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("operator test ref")
}

fn declared_binding() -> ConsistencyGroupBinding {
    canonical_consistency_group_binding(ConsistencyGroupBindingInput {
        group_id: "group:operator".to_string(),
        extension_id: "extension-operator".to_string(),
        service_id: "service-operator".to_string(),
        service_generation: INITIAL_CONSISTENCY_EPOCH,
        application_manifest_ref: test_ref("application"),
        engine_algorithm_profile: "raft".to_string(),
        engine_implementation_profile: "live-raft-static-v1".to_string(),
        membership_ref: test_ref("membership"),
        config_epoch: INITIAL_CONSISTENCY_EPOCH,
        placement_ref: test_ref("placement"),
        fencing_ref: test_ref("fencing"),
        fencing_epoch: INITIAL_CONSISTENCY_EPOCH,
        resource_profile_ref: test_ref("resources"),
        policy_refs: vec![test_ref("policy")],
        non_claims: vec!["operator-readback-does-not-prove-quorum-currentness".to_string()],
        supported_read_modes: vec![ConsistencyReadMode::Linearizable],
        max_command_bytes: MAX_COMMAND_BYTES,
        max_in_flight_operations: MAX_IN_FLIGHT,
    })
    .expect("operator binding")
}

fn active_binding() -> ConsistencyGroupBinding {
    let declared = declared_binding();
    let plan = plan_consistency_operation(
        &declared,
        command_for(&declared, ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        }),
    )
    .expect("operator open plan");
    let outcome = normalize_consistency_outcome(
        &declared,
        &plan,
        success_input(&declared, &plan, ConsistencyOutcomeKind::Opened),
    )
    .expect("operator open outcome");
    apply_consistency_outcome(&declared, &plan, &outcome).expect("active operator binding")
}

fn draining_binding() -> ConsistencyGroupBinding {
    let active = active_binding();
    let plan = plan_consistency_operation(&active, command_for(&active, ConsistencyOperation::Drain))
        .expect("operator drain plan");
    let outcome =
        normalize_consistency_outcome(&active, &plan, success_input(&active, &plan, ConsistencyOutcomeKind::Drained))
            .expect("operator drain outcome");
    apply_consistency_outcome(&active, &plan, &outcome).expect("draining operator binding")
}

fn success_input(
    binding: &ConsistencyGroupBinding,
    plan: &ConsistencyPortPlan,
    kind: ConsistencyOutcomeKind,
) -> ConsistencyOutcomeInput {
    ConsistencyOutcomeInput {
        request_ref: plan.request_ref.clone(),
        binding_ref: binding.binding_ref.clone(),
        service_generation: binding.service_generation,
        config_epoch: binding.config_epoch,
        fencing_epoch: binding.fencing_epoch,
        kind,
        result_ref: Some(test_ref("operator-success-result")),
        evidence_refs: vec![test_ref("operator-success-evidence")],
        diagnostics: Vec::new(),
    }
}

fn command_for(binding: &ConsistencyGroupBinding, operation: ConsistencyOperation) -> ConsistencyPortCommandInput {
    ConsistencyPortCommandInput {
        request_ref: test_ref("operator-request"),
        binding_ref: binding.binding_ref.clone(),
        group_id: binding.group_id.clone(),
        extension_id: binding.extension_id.clone(),
        service_id: binding.service_id.clone(),
        service_generation: binding.service_generation,
        application_manifest_ref: binding.application_manifest_ref.clone(),
        engine_algorithm_profile: binding.engine_algorithm_profile.clone(),
        engine_implementation_profile: binding.engine_implementation_profile.clone(),
        membership_ref: binding.membership_ref.clone(),
        config_epoch: binding.config_epoch,
        placement_ref: binding.placement_ref.clone(),
        fencing_ref: binding.fencing_ref.clone(),
        fencing_epoch: binding.fencing_epoch,
        resource_profile_ref: binding.resource_profile_ref.clone(),
        policy_refs: binding.policy_refs.clone(),
        authority_refs: vec![test_ref("operator-authority")],
        observed_in_flight_operations: 0,
        operation,
    }
}

fn health() -> ReplicaAggregateHealthEvidence {
    ReplicaAggregateHealthEvidence {
        status: "healthy".to_string(),
        selected_record_count: 0,
        suppressed_heartbeat_count: 0,
        saturated: false,
        diagnostic: None,
        evidence_ref: test_ref("aggregate-health"),
        production_admitted: false,
    }
}
