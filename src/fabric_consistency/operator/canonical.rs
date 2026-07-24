use super::*;

pub(super) fn preflight_ref(
    action: ConsistencyOperatorAction,
    dry_run: bool,
    plan: &ConsistencyPortPlan,
) -> Result<String> {
    crate::preserves_rail::validate_content_ref(&plan.plan_ref)?;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "fabric-consistency-operator-preflight-v1",
        vec![
            crate::preserves_rail::string(action.as_str()),
            crate::preserves_rail::bool_value(dry_run),
            crate::preserves_rail::string(&plan.plan_ref),
            crate::preserves_rail::string(plan.decision.as_str()),
            crate::preserves_rail::string(plan.lifecycle_before.as_str()),
            crate::preserves_rail::string(plan.lifecycle_after.as_str()),
        ],
    ))
}

pub(super) fn execution_ref(
    preflight: &ConsistencyOperatorPreflight,
    status: ConsistencyOperatorExecutionStatus,
    effect_ref: Option<&str>,
) -> Result<String> {
    let effect = match effect_ref {
        Some(reference) => {
            crate::preserves_rail::validate_content_ref(reference)?;
            crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)])
        }
        None => crate::preserves_rail::record("none", Vec::new()),
    };
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "fabric-consistency-operator-execution-v1",
        vec![
            crate::preserves_rail::string(&preflight.preflight_ref),
            crate::preserves_rail::string(preflight.action.as_str()),
            crate::preserves_rail::string(status.as_str()),
            effect,
        ],
    ))
}

pub(super) fn readback_ref(
    binding: &ConsistencyGroupBinding,
    replica: &ConsistencyOperatorReplicaState,
    evidence_refs: &[String],
    evidence_truncated: bool,
    health: &ReplicaAggregateHealthEvidence,
) -> Result<String> {
    for reference in evidence_refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "fabric-consistency-operator-readback-v1",
        vec![
            crate::preserves_rail::string(&binding.binding_ref),
            crate::preserves_rail::string(&binding.group_id),
            crate::preserves_rail::string(&binding.extension_id),
            crate::preserves_rail::string(&binding.service_id),
            crate::preserves_rail::u64_value(binding.service_generation),
            crate::preserves_rail::string(binding.lifecycle.as_str()),
            crate::preserves_rail::string(&replica.node_id),
            crate::preserves_rail::string(replica.role.as_str()),
            crate::preserves_rail::string(replica.lifecycle.as_str()),
            crate::preserves_rail::u64_value(replica.term),
            crate::preserves_rail::u64_value(replica.commit_index),
            crate::preserves_rail::u64_value(replica.last_applied),
            crate::preserves_rail::sequence(evidence_refs.iter().map(crate::preserves_rail::string).collect()),
            crate::preserves_rail::bool_value(evidence_truncated),
            crate::preserves_rail::string(&health.evidence_ref),
            crate::preserves_rail::bool_value(health.production_admitted),
        ],
    ))
}
