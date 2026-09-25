use std::collections::VecDeque;

use basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON;
use basalt::world_branch_authority::parse_branch_authority_policy;
use molten_core::world_branch_authority::*;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_promotion::*;

use super::*;
use crate::error::Result;

const POLICY_GENERATION: u64 = 1;
const SOURCE_GENERATION: u64 = 9;
const DESTINATION_GENERATION: u64 = SOURCE_GENERATION + 1;
const SOURCE_LIMIT: u64 = 100;
const DESTINATION_LIMIT: u64 = 50;

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn content_ref(label: &str) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-branch-authority.shell-test.v1");
    hasher.update(label.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn scope(resource: &str, abilities: &[&str], limit: u64) -> NormalizedCapabilityScope {
    NormalizedCapabilityScope::new(resource, abilities.iter().map(|ability| (*ability).to_string()), Some(limit))
        .expect("valid test scope")
}

fn facts(kind: CapabilityKind, action: WorldBranchAction) -> WorldBranchAuthorityFacts {
    let source_scope = scope("service/root", &["read", "write"], SOURCE_LIMIT);
    let destination_scope = if kind == CapabilityKind::ScopedService {
        scope("service/root/child", &["read"], DESTINATION_LIMIT)
    } else {
        source_scope.clone()
    };
    WorldBranchAuthorityFacts {
        capability_kind: kind,
        action,
        source_branch_ref: content_ref("source"),
        destination_branch_ref: content_ref("destination"),
        capability_ref: content_ref("capability"),
        source_scope,
        destination_scope,
        policy_generation: POLICY_GENERATION,
        mapping_lossless: true,
    }
}

fn current(label: &str) -> CurrentAuthorityFacts {
    CurrentAuthorityFacts {
        observation_ref: content_ref(label),
        policy_current: true,
        capability_current: true,
        revocation_current: true,
        replay_current: true,
        scope_current: true,
        ucan_verified: true,
    }
}

struct Runtime {
    current: VecDeque<CurrentAuthorityFacts>,
    transfer_unknown: bool,
    reconcile_available: bool,
    simulation_deterministic: bool,
    activation_policy_current: bool,
    post_transfer_source_active: bool,
    post_transfer_generation: u64,
    activation_unknown: bool,
    activation_reconcile_outcome: ActivationOutcome,
    policy_calls: usize,
    ownership_calls: usize,
    grant_calls: usize,
    simulation_calls: usize,
    promotion_calls: usize,
    promotion_dispatch_overclaim: bool,
    promotion_uncommitted: bool,
    transfer_calls: usize,
    activation_calls: usize,
    activation_reconcile_calls: usize,
    receipt_bytes: Vec<Vec<u8>>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            current: VecDeque::from([current("initial"), current("activation")]),
            transfer_unknown: false,
            reconcile_available: true,
            simulation_deterministic: true,
            activation_policy_current: true,
            post_transfer_source_active: false,
            post_transfer_generation: DESTINATION_GENERATION,
            activation_unknown: false,
            activation_reconcile_outcome: ActivationOutcome::Activated,
            policy_calls: 0,
            ownership_calls: 0,
            grant_calls: 0,
            simulation_calls: 0,
            promotion_calls: 0,
            promotion_dispatch_overclaim: false,
            promotion_uncommitted: false,
            transfer_calls: 0,
            activation_calls: 0,
            activation_reconcile_calls: 0,
            receipt_bytes: Vec::new(),
        }
    }

    fn realization(
        plan: &WorldBranchAuthorityPlan,
        operation_ref: String,
        source_active: bool,
        transfer_generation: Option<u64>,
    ) -> WorldBranchRealizationObservation {
        WorldBranchRealizationObservation {
            plan_ref: plan.plan_ref.clone(),
            policy_ref: plan.policy_ref.clone(),
            capability_ref: plan.capability_ref.clone(),
            operation_ref,
            evidence_refs: plan
                .obligations
                .iter()
                .enumerate()
                .filter(|(_, obligation)| **obligation != WorldBranchObligation::DenyActivation)
                .map(|(index, _)| content_ref(&format!("evidence-{index}")))
                .collect(),
            destination_scope: plan.destination_scope.clone(),
            destination_grant_current: true,
            source_active,
            destination_active: false,
            transfer_generation,
            simulation_adapter_ref: (plan.mode == Some(WorldBranchMode::SimulationOnly))
                .then(|| content_ref("simulation-adapter")),
            simulation_adapter_deterministic: plan.mode == Some(WorldBranchMode::SimulationOnly),
            release_reservation_ref: None,
            promotion_admission: None,
            bearer_material_present: false,
            receipt_claims_authority: false,
        }
    }
}

fn promotion_material(
    plan: &WorldBranchAuthorityPlan,
) -> (WorldPromotionPlan, Vec<WorldReleaseReservation>, WorldReleaseReservationRef) {
    let policy_ref = WorldHeadPolicyRef::new(content_ref("promotion-policy")).expect("policy ref");
    let request = WorldPromotionRequest {
        operation_ref: WorldPromotionOperationRef::new(content_ref("promotion-operation")).expect("operation ref"),
        branch_id: WorldBranchId::new("release").expect("branch id"),
        branch_class: WorldBranchClass::Candidate,
        expected_head: WorldCommitRef::new(content_ref("promotion-active")).expect("active head"),
        candidate_head: WorldCommitRef::new(content_ref("promotion-candidate")).expect("candidate head"),
        expected_generation: POLICY_GENERATION,
        policy_ref: policy_ref.clone(),
        authority: WorldPromotionAuthorityObservation {
            authority_ref: WorldPromotionAuthorityRef::new(content_ref("promotion-authority")).expect("authority ref"),
            policy_ref,
            observed_generation: POLICY_GENERATION,
            admitted: true,
        },
        intent_closure_complete: true,
        simulation_only: false,
        intents: vec![WorldEffectIntent {
            intent_ref: WorldEffectIntentRef::new(content_ref("promotion-intent")).expect("intent ref"),
            semantic_ref: WorldSemanticIntentRef::new(content_ref("promotion-semantic")).expect("semantic ref"),
            handler_ref: WorldPromotionHandlerRef::new(content_ref("promotion-handler")).expect("handler ref"),
            adapter_ref: WorldPromotionAdapterRef::new(content_ref("promotion-adapter")).expect("adapter ref"),
            release_class: Some(WorldIntentReleaseClass::Release),
        }],
        bounds: WorldPromotionBounds::standard(),
    };
    let promotion = plan_world_promotion(&request).expect("promotion plan");
    let mut committed = promotion.reservations.clone();
    for reservation in &mut committed {
        reservation.state = WorldReleaseState::Committed;
    }
    let selected_ref = committed[0].reservation_ref.clone();
    assert_eq!(plan.mode, Some(WorldBranchMode::PromotionGated));
    (promotion, committed, selected_ref)
}

impl CurrentBranchPolicyPort for Runtime {
    fn observe_policy(&mut self) -> Result<CurrentPolicyObservation> {
        let policy =
            parse_branch_authority_policy(DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON).expect("valid Basalt policy");
        self.policy_calls += 1;
        Ok(CurrentPolicyObservation {
            policy_json: DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON.to_string(),
            policy_ref: policy.identity().to_string(),
            generation: policy.generation(),
            current: self.policy_calls == 1 || self.activation_policy_current,
        })
    }
}

impl CurrentBranchAuthorityPort for Runtime {
    fn observe_authority(&mut self, _facts: &WorldBranchAuthorityFacts) -> Result<CurrentAuthorityFacts> {
        Ok(self.current.pop_front().expect("bounded authority observation"))
    }
}

impl DestinationGrantPort for Runtime {
    fn realize_destination_grant(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
    ) -> Result<WorldBranchRealizationObservation> {
        self.grant_calls += 1;
        Ok(Self::realization(plan, content_ref("grant-operation"), true, None))
    }
}

impl LinearAuthorityTransferPort for Runtime {
    fn observe_ownership(&mut self, plan: &WorldBranchAuthorityPlan) -> Result<LinearOwnershipObservation> {
        self.ownership_calls += 1;
        let is_initial = self.ownership_calls == 1;
        Ok(LinearOwnershipObservation {
            capability_ref: plan.capability_ref.clone(),
            generation: if is_initial {
                SOURCE_GENERATION
            } else {
                self.post_transfer_generation
            },
            source_active: is_initial || self.post_transfer_source_active,
            destination_active: false,
            observation_ref: content_ref(if is_initial {
                "ownership-initial"
            } else {
                "ownership-activation"
            }),
        })
    }

    fn transfer(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
        expected_generation: u64,
        operation_ref: &str,
    ) -> Result<LinearTransferOutcome> {
        self.transfer_calls += 1;
        if self.transfer_unknown {
            Ok(LinearTransferOutcome::Unknown)
        } else {
            Ok(LinearTransferOutcome::Committed(Box::new(Self::realization(
                plan,
                operation_ref.to_string(),
                false,
                expected_generation.checked_add(1),
            ))))
        }
    }

    fn reconcile_transfer(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
        operation_ref: &str,
    ) -> Result<Option<WorldBranchRealizationObservation>> {
        Ok(self
            .reconcile_available
            .then(|| Self::realization(plan, operation_ref.to_string(), false, Some(DESTINATION_GENERATION))))
    }
}

impl SimulationAuthorityPort for Runtime {
    fn bind_simulation(&mut self, plan: &WorldBranchAuthorityPlan) -> Result<WorldBranchRealizationObservation> {
        self.simulation_calls += 1;
        let mut observation = Self::realization(plan, content_ref("simulation-operation"), true, None);
        observation.simulation_adapter_deterministic = self.simulation_deterministic;
        Ok(observation)
    }
}
