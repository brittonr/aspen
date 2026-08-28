use std::collections::VecDeque;

use basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON;
use basalt::world_branch_authority::parse_branch_authority_policy;
use molten_core::world_branch_authority::*;

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
            bearer_material_present: false,
            receipt_claims_authority: false,
        }
    }
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
        let initial = self.ownership_calls == 1;
        Ok(LinearOwnershipObservation {
            capability_ref: plan.capability_ref.clone(),
            generation: if initial {
                SOURCE_GENERATION
            } else {
                self.post_transfer_generation
            },
            source_active: initial || self.post_transfer_source_active,
            destination_active: false,
            observation_ref: content_ref(if initial {
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

impl BranchActivationPort for Runtime {
    fn activate(&mut self, _decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        self.activation_calls += 1;
        if self.activation_unknown {
            Ok(ActivationOutcome::Unknown)
        } else {
            Ok(ActivationOutcome::Activated)
        }
    }

    fn reconcile_activation(&mut self, _decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        self.activation_reconcile_calls += 1;
        Ok(self.activation_reconcile_outcome)
    }
}

impl BranchAuthorityReceiptPort for Runtime {
    fn publish_receipt(&mut self, _receipt_ref: &str, canonical_json: &[u8]) -> Result<()> {
        self.receipt_bytes.push(canonical_json.to_vec());
        Ok(())
    }
}

// r[verify molten.world_branch_authority.verification]
#[test]
fn copyable_and_simulation_paths_use_only_their_admitted_ports() {
    let mut copy_runtime = Runtime::new();
    let copy = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut copy_runtime,
    )
    .expect("copyable execution");
    assert_eq!(copy.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(copy_runtime.receipt_bytes.len(), 3);
    assert_eq!(copy_runtime.grant_calls, 1);
    assert_eq!(copy_runtime.simulation_calls, 0);

    let mut simulation_runtime = Runtime::new();
    let simulation = execute_world_branch_authority(
        &facts(CapabilityKind::ExternalEffect, WorldBranchAction::Simulate),
        &mut simulation_runtime,
    )
    .expect("simulation execution");
    assert_eq!(simulation.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(simulation_runtime.simulation_calls, 1);
    assert_eq!(simulation_runtime.grant_calls, 0);

    let mut fallback_runtime = Runtime::new();
    fallback_runtime.simulation_deterministic = false;
    let fallback = execute_world_branch_authority(
        &facts(CapabilityKind::ExternalEffect, WorldBranchAction::Simulate),
        &mut fallback_runtime,
    )
    .expect("live fallback returns a denial decision");
    assert_eq!(
        fallback.activation.expect("fallback decision").diagnostic,
        WorldBranchAuthorityDiagnostic::SimulationLiveFallback
    );
    assert_eq!(fallback_runtime.activation_calls, 0);
}

#[test]
fn linear_unknown_outcome_reconciles_without_blind_retry() {
    let mut runtime = Runtime::new();
    runtime.transfer_unknown = true;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut runtime,
    )
    .expect("reconciled transfer");
    assert_eq!(execution.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(runtime.transfer_calls, 1);
    assert_eq!(runtime.ownership_calls, 2);

    let mut unresolved = Runtime::new();
    unresolved.transfer_unknown = true;
    unresolved.reconcile_available = false;
    let error = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut unresolved,
    )
    .expect_err("unknown transfer denied");
    assert!(error.to_string().contains("outcome is unknown"));
    assert_eq!(unresolved.transfer_calls, 1);
    assert_eq!(unresolved.activation_calls, 0);

    let mut ambiguous = Runtime::new();
    ambiguous.post_transfer_source_active = true;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut ambiguous,
    )
    .expect("fresh ownership ambiguity emits a denial decision");
    assert_eq!(
        execution.activation.expect("linear denial").diagnostic,
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
    );
    assert_eq!(ambiguous.activation_calls, 0);

    let mut stale_generation = Runtime::new();
    stale_generation.post_transfer_generation = SOURCE_GENERATION;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut stale_generation,
    )
    .expect("stale ownership generation emits a denial decision");
    assert_eq!(
        execution.activation.expect("stale generation denial").diagnostic,
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
    );
    assert_eq!(stale_generation.activation_calls, 0);
}

#[test]
fn activation_rechecks_policy_and_reconciles_unknown_outcomes() {
    let mut stale_policy = Runtime::new();
    stale_policy.activation_policy_current = false;
    let stale = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut stale_policy,
    )
    .expect("stale activation policy emits a denial decision");
    assert_eq!(
        stale.activation.expect("stale policy decision").diagnostic,
        WorldBranchAuthorityDiagnostic::PolicyStale
    );
    assert_eq!(stale_policy.policy_calls, 2);
    assert_eq!(stale_policy.activation_calls, 0);

    let mut reconciled = Runtime::new();
    reconciled.activation_unknown = true;
    let activation = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut reconciled,
    )
    .expect("unknown activation is observed before completion");
    assert_eq!(activation.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(reconciled.activation_calls, 1);
    assert_eq!(reconciled.activation_reconcile_calls, 1);

    let mut unresolved = Runtime::new();
    unresolved.activation_unknown = true;
    unresolved.activation_reconcile_outcome = ActivationOutcome::Unknown;
    let activation = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut unresolved,
    )
    .expect("unresolved activation remains explicit");
    assert_eq!(activation.activation_outcome, Some(ActivationOutcome::Unknown));
    assert_eq!(unresolved.activation_calls, 1);
    assert_eq!(unresolved.activation_reconcile_calls, 1);
    let receipt = unresolved.receipt_bytes.last().expect("unknown activation outcome receipt");
    crate::preserves_rail::strict_canonical_decode(receipt).expect("canonical activation outcome receipt");
    assert!(contains_bytes(receipt, b"activation-outcome"));
    assert!(contains_bytes(receipt, b"unknown"));
    assert!(!contains_bytes(receipt, b"secret="));
}

#[test]
fn bearer_and_promotion_modes_never_activate_from_policy_receipts() {
    let mut bearer_runtime = Runtime::new();
    let bearer = execute_world_branch_authority(
        &facts(CapabilityKind::BearerCredential, WorldBranchAction::Create),
        &mut bearer_runtime,
    )
    .expect("metadata-only denial");
    assert!(!bearer.plan.allowed);
    assert_eq!(bearer_runtime.activation_calls, 0);
    assert_eq!(bearer_runtime.receipt_bytes.len(), 1);

    let mut promotion_runtime = Runtime::new();
    let promotion = execute_world_branch_authority(
        &facts(CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        &mut promotion_runtime,
    )
    .expect("promotion remains pending");
    assert_eq!(
        promotion.activation.expect("activation decision").diagnostic,
        WorldBranchAuthorityDiagnostic::MissingObligationEvidence
    );
    assert_eq!(promotion_runtime.activation_calls, 0);
    assert!(promotion_runtime.receipt_bytes.iter().all(|bytes| {
        let text = String::from_utf8_lossy(bytes);
        !text.contains("secret=") && !text.contains("bearer-token")
    }));
}
