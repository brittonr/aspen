mod negative;
mod positive;

use super::*;

const SERVICE_GENERATION: u64 = INITIAL_CONSISTENCY_EPOCH;
const CONFIG_EPOCH: u64 = INITIAL_CONSISTENCY_EPOCH;
const FENCING_EPOCH: u64 = INITIAL_CONSISTENCY_EPOCH;
const NEXT_CONFIG_EPOCH: u64 = CONFIG_EPOCH + NEXT_CONSISTENCY_EPOCH_STEP;
const MAX_COMMAND_BYTES: u64 = 4_096;
const MAX_IN_FLIGHT: u32 = 8;
const COMMAND_BYTES: u64 = 256;

fn test_ref(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-consistency-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("test ref")
}

fn binding_input() -> ConsistencyGroupBindingInput {
    ConsistencyGroupBindingInput {
        group_id: "group:extension-a".to_string(),
        extension_id: "extension-a".to_string(),
        service_id: "service-a".to_string(),
        service_generation: SERVICE_GENERATION,
        application_manifest_ref: test_ref("application-manifest"),
        engine_algorithm_profile: "raft".to_string(),
        engine_implementation_profile: "live-raft-static-v1".to_string(),
        membership_ref: test_ref("membership"),
        config_epoch: CONFIG_EPOCH,
        placement_ref: test_ref("placement"),
        fencing_ref: test_ref("fencing"),
        fencing_epoch: FENCING_EPOCH,
        resource_profile_ref: test_ref("resource-profile"),
        policy_refs: vec![test_ref("policy")],
        non_claims: vec![
            "consistency-does-not-prove-extension-semantics".to_string(),
            "local-status-is-not-quorum-currentness".to_string(),
        ],
        supported_read_modes: vec![ConsistencyReadMode::LocalStale, ConsistencyReadMode::Linearizable],
        max_command_bytes: MAX_COMMAND_BYTES,
        max_in_flight_operations: MAX_IN_FLIGHT,
    }
}

fn declared_binding() -> ConsistencyGroupBinding {
    canonical_consistency_group_binding(binding_input()).expect("declared binding")
}

fn command_for(binding: &ConsistencyGroupBinding, operation: ConsistencyOperation) -> ConsistencyPortCommandInput {
    ConsistencyPortCommandInput {
        request_ref: test_ref("request"),
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
        authority_refs: vec![test_ref("authority")],
        observed_in_flight_operations: 0,
        operation,
    }
}

fn active_binding() -> ConsistencyGroupBinding {
    let declared = declared_binding();
    let plan = plan_consistency_operation(
        &declared,
        command_for(&declared, ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        }),
    )
    .expect("open plan");
    assert!(plan.admitted());
    let outcome = normalized_success(&declared, &plan, ConsistencyOutcomeKind::Opened);
    apply_consistency_outcome(&declared, &plan, &outcome).expect("active binding")
}

fn normalized_success(
    binding: &ConsistencyGroupBinding,
    plan: &ConsistencyPortPlan,
    kind: ConsistencyOutcomeKind,
) -> ConsistencyPortOutcome {
    normalize_consistency_outcome(binding, plan, ConsistencyOutcomeInput {
        request_ref: plan.request_ref.clone(),
        binding_ref: plan.binding_ref.clone(),
        service_generation: binding.service_generation,
        config_epoch: binding.config_epoch,
        fencing_epoch: binding.fencing_epoch,
        kind,
        result_ref: Some(test_ref("result")),
        evidence_refs: vec![test_ref("evidence")],
        diagnostics: Vec::new(),
    })
    .expect("normalized success")
}

fn normalized_denial(binding: &ConsistencyGroupBinding, plan: &ConsistencyPortPlan) -> ConsistencyPortOutcome {
    normalize_consistency_outcome(binding, plan, ConsistencyOutcomeInput {
        request_ref: plan.request_ref.clone(),
        binding_ref: plan.binding_ref.clone(),
        service_generation: binding.service_generation,
        config_epoch: binding.config_epoch,
        fencing_epoch: binding.fencing_epoch,
        kind: ConsistencyOutcomeKind::Denied,
        result_ref: None,
        evidence_refs: vec![test_ref("denial-evidence")],
        diagnostics: vec!["operation-denied".to_string()],
    })
    .expect("normalized denial")
}
