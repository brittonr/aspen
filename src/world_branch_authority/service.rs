use molten_core::world_branch_authority::WorldBranchAction;
use molten_core::world_branch_authority::WorldBranchActivationDecision;
use molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic;
use molten_core::world_branch_authority::WorldBranchAuthorityFacts;
use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchMode;
use molten_core::world_branch_authority::WorldBranchRealizationObservation;
use molten_core::world_branch_authority::decide_world_branch_activation;
use molten_core::world_branch_authority::deny_world_branch_authority_plan;
use molten_core::world_branch_authority::plan_world_branch_authority;
use molten_core::world_branch_authority::valid_content_ref;

use super::ports::*;
use super::records::*;
use crate::error::MoltenError;
use crate::error::Result;

const LINEAR_OPERATION_DOMAIN: &str = "onixresearch.molten.world-branch-authority.linear-operation.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBranchAuthorityExecution {
    pub plan: WorldBranchAuthorityPlan,
    pub realization: Option<WorldBranchRealizationObservation>,
    pub activation: Option<WorldBranchActivationDecision>,
    pub activation_outcome: Option<ActivationOutcome>,
    pub receipt_refs: Vec<String>,
}

// r[impl molten.world_branch_authority.activation]
// r[impl molten.world_branch_authority.linear]
// r[impl molten.world_branch_authority.simulation]
pub fn execute_world_branch_authority<R: WorldBranchAuthorityRuntime>(
    facts: &WorldBranchAuthorityFacts,
    runtime: &mut R,
) -> Result<WorldBranchAuthorityExecution> {
    let policy_observation = runtime.observe_policy()?;
    let initial_current = runtime.observe_authority(facts)?;
    let mut plan = plan_world_branch_authority(policy_observation.policy_json.as_str(), facts, &initial_current);
    if !policy_observation.current
        || policy_observation.generation != facts.policy_generation
        || !valid_content_ref(&policy_observation.policy_ref)
        || (plan.allowed && policy_observation.policy_ref != plan.policy_ref)
    {
        plan = deny_world_branch_authority_plan(plan, WorldBranchAuthorityDiagnostic::PolicyStale);
    }

    let mut receipt_refs = Vec::new();
    publish(runtime, &plan_receipt(&plan), &mut receipt_refs)?;
    if !plan.allowed {
        return Ok(WorldBranchAuthorityExecution {
            plan,
            realization: None,
            activation: None,
            activation_outcome: None,
            receipt_refs,
        });
    }

    let mut realization = realize(&plan, runtime)?;
    if plan.mode == Some(WorldBranchMode::Linear) {
        recheck_linear_ownership(&plan, &mut realization, runtime)?;
    }
    let activation_policy = runtime.observe_policy()?;
    let mut activation_current = runtime.observe_authority(facts)?;
    if !policy_matches_plan(&activation_policy, &plan, facts.policy_generation) {
        activation_current.policy_current = false;
    }
    let activation = decide_world_branch_activation(&plan, &realization, &activation_current);
    publish(runtime, &activation_receipt(&plan, &realization, &activation), &mut receipt_refs)?;
    let activation_outcome = if activation.allowed {
        let attempted = runtime.activate(&activation)?;
        let observed = if attempted == ActivationOutcome::Unknown {
            runtime.reconcile_activation(&activation)?
        } else {
            attempted
        };
        Some(observed)
    } else {
        None
    };
    if let Some(outcome) = activation_outcome {
        publish(runtime, &activation_outcome_receipt(&plan, &realization, &activation, outcome), &mut receipt_refs)?;
    }
    Ok(WorldBranchAuthorityExecution {
        plan,
        realization: Some(realization),
        activation: Some(activation),
        activation_outcome,
        receipt_refs,
    })
}

fn policy_matches_plan(
    policy: &CurrentPolicyObservation,
    plan: &WorldBranchAuthorityPlan,
    expected_generation: u64,
) -> bool {
    policy.current
        && policy.generation == expected_generation
        && policy.policy_ref == plan.policy_ref
        && valid_content_ref(&policy.policy_ref)
}

fn recheck_linear_ownership<R: LinearAuthorityTransferPort>(
    plan: &WorldBranchAuthorityPlan,
    realization: &mut WorldBranchRealizationObservation,
    runtime: &mut R,
) -> Result<()> {
    let ownership = runtime.observe_ownership(plan)?;
    let expected_generation = realization.transfer_generation;
    let current = ownership.capability_ref == plan.capability_ref
        && Some(ownership.generation) == expected_generation
        && !ownership.source_active
        && !ownership.destination_active
        && valid_content_ref(&ownership.observation_ref);
    if current {
        realization.source_active = ownership.source_active;
        realization.destination_active = ownership.destination_active;
        realization.transfer_generation = Some(ownership.generation);
    } else {
        realization.source_active = true;
        realization.transfer_generation = None;
    }
    Ok(())
}

fn realize<R: WorldBranchAuthorityRuntime>(
    plan: &WorldBranchAuthorityPlan,
    runtime: &mut R,
) -> Result<WorldBranchRealizationObservation> {
    match plan.mode {
        Some(WorldBranchMode::Copyable | WorldBranchMode::Attenuated | WorldBranchMode::ReplaceBeforeActivation) => {
            runtime.realize_destination_grant(plan)
        }
        Some(WorldBranchMode::Linear) => realize_linear(plan, runtime),
        Some(WorldBranchMode::SimulationOnly) => runtime.bind_simulation(plan),
        Some(WorldBranchMode::PromotionGated) => realize_promotion(plan, runtime),
        Some(WorldBranchMode::NonBranchable) | None => {
            Err(MoltenError::invalid_harness("branch-authority denied plan cannot be realized"))
        }
    }
}

fn realize_linear<R: WorldBranchAuthorityRuntime>(
    plan: &WorldBranchAuthorityPlan,
    runtime: &mut R,
) -> Result<WorldBranchRealizationObservation> {
    let ownership = runtime.observe_ownership(plan)?;
    if ownership.capability_ref != plan.capability_ref
        || ownership.generation == 0
        || !ownership.source_active
        || ownership.destination_active
        || !valid_content_ref(&ownership.observation_ref)
    {
        return Err(MoltenError::invalid_harness("linear branch-authority ownership is stale or ambiguous"));
    }
    let operation_ref = linear_operation_ref(plan, ownership.generation);
    let successor_generation = ownership
        .generation
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("linear branch-authority generation overflow"))?;
    let observation = match runtime.transfer(plan, ownership.generation, operation_ref.as_str())? {
        LinearTransferOutcome::Committed(observation) => *observation,
        LinearTransferOutcome::Denied => {
            return Err(MoltenError::invalid_harness("linear branch-authority transfer was denied"));
        }
        LinearTransferOutcome::Unknown => runtime
            .reconcile_transfer(plan, operation_ref.as_str())?
            .ok_or_else(|| MoltenError::invalid_harness("linear branch-authority transfer outcome is unknown"))?,
    };
    if observation.operation_ref != operation_ref
        || observation.transfer_generation != Some(successor_generation)
        || observation.source_active
        || observation.destination_active
    {
        return Err(MoltenError::invalid_harness(
            "linear branch-authority transfer observation is crossed or ambiguous",
        ));
    }
    Ok(observation)
}

fn realize_promotion<R: PromotionReservationPort>(
    plan: &WorldBranchAuthorityPlan,
    runtime: &mut R,
) -> Result<WorldBranchRealizationObservation> {
    let admission = runtime.admit_promotion_reservation(plan)?;
    let evidence_refs = vec![
        admission.promotion_plan_ref.clone(),
        admission.reservation_ref.clone(),
        admission.admission_ref.clone(),
    ];
    Ok(WorldBranchRealizationObservation {
        plan_ref: plan.plan_ref.clone(),
        policy_ref: plan.policy_ref.clone(),
        capability_ref: plan.capability_ref.clone(),
        operation_ref: admission.admission_ref.clone(),
        evidence_refs,
        destination_scope: plan.destination_scope.clone(),
        destination_grant_current: admission.reservation_committed && admission.complete_reservation_set,
        source_active: true,
        destination_active: false,
        transfer_generation: None,
        simulation_adapter_ref: None,
        simulation_adapter_deterministic: false,
        release_reservation_ref: Some(admission.reservation_ref.clone()),
        promotion_admission: Some(admission),
        bearer_material_present: false,
        receipt_claims_authority: false,
    })
}

fn linear_operation_ref(plan: &WorldBranchAuthorityPlan, generation: u64) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(LINEAR_OPERATION_DOMAIN);
    for value in [plan.plan_ref.as_str(), plan.capability_ref.as_str()] {
        let length = u64::try_from(value.len()).unwrap_or(u64::MAX);
        hasher.update(&length.to_le_bytes());
        hasher.update(value.as_bytes());
    }
    hasher.update(&generation.to_le_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn publish<R: BranchAuthorityReceiptPort>(
    runtime: &mut R,
    receipt: &WorldBranchAuthorityReceipt,
    receipt_refs: &mut Vec<String>,
) -> Result<()> {
    let (receipt_ref, bytes) = encode_receipt(receipt)?;
    runtime.publish_receipt(receipt_ref.as_str(), &bytes)?;
    receipt_refs.push(receipt_ref);
    Ok(())
}

pub const fn action_requires_promotion(action: WorldBranchAction) -> bool {
    matches!(action, WorldBranchAction::Promote)
}
