//! The branch-authority shell ports the qualification node serves: current
//! policy and authority observation, destination grants, activation and
//! receipts, with linear transfer, simulation and promotion failing closed.

use ed25519_dalek::Signer;

use crate::world_branch_authority::BranchActivationPort;
use crate::world_branch_authority::BranchAuthorityReceiptPort;
use crate::world_branch_authority::CurrentBranchAuthorityPort;
use crate::world_branch_authority::CurrentBranchPolicyPort;
use crate::world_branch_authority::DestinationGrantPort;
use crate::world_branch_authority::LinearAuthorityTransferPort;
use crate::world_branch_authority::PromotionReservationPort;
use crate::world_branch_authority::SimulationAuthorityPort;

impl CurrentBranchPolicyPort for super::state::BranchRuntime {
    fn observe_policy(&mut self) -> crate::error::Result<crate::world_branch_authority::CurrentPolicyObservation> {
        let policy_json = self
            .root
            .read_to_string(&crate::node_state::RelativePath::parse(super::POLICY_PATH)?, super::MAX_STATE_BYTES)?;
        let policy = basalt::world_branch_authority::parse_branch_authority_policy(&policy_json)
            .map_err(|_| super::super::invalid("current branch-authority policy document is malformed"))?;
        Ok(crate::world_branch_authority::CurrentPolicyObservation {
            policy_ref: policy.identity().to_string(),
            generation: policy.generation(),
            current: true,
            policy_json,
        })
    }
}

impl CurrentBranchAuthorityPort for super::state::BranchRuntime {
    fn observe_authority(
        &mut self,
        facts: &molten_core::world_branch_authority::WorldBranchAuthorityFacts,
    ) -> crate::error::Result<molten_core::world_branch_authority::CurrentAuthorityFacts> {
        self.tick = self.tick.checked_add(1).ok_or_else(|| super::super::invalid("authority tick overflowed"))?;
        let at_tick = self.tick;
        let revoked = self.revoked_at(at_tick)?;
        let policy = self.observe_policy()?;
        let token = &self.token;
        let proofset = crate::capability_tokens::CapabilityProofset {
            holder_ref: token.holder_ref.clone(),
            session_ref: token.session_ref.clone(),
            context_ref: token.context_ref.clone(),
            tokens: vec![token.clone()],
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            revocation_refs: revoked.clone(),
            evidence_refs: Vec::new(),
        };
        let request = crate::capability_tokens::CapabilityRequest {
            holder_ref: token.holder_ref.clone(),
            session_ref: token.session_ref.clone(),
            context_ref: facts.source_branch_ref.clone(),
            resource_ref: facts.capability_ref.clone(),
            ability: super::BRANCH_ABILITY.to_string(),
            scope: facts.destination_scope.resource().to_string(),
            at_tick,
            required_policy_refs: Vec::new(),
            required_resource_refs: Vec::new(),
            required_token_kind: Some(super::TOKEN_KIND.to_string()),
            caveat_context: Vec::new(),
        };
        let admission = crate::capability_tokens::admit_capability(&proofset, &request)?;
        let current = molten_core::world_branch_authority::CurrentAuthorityFacts {
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-qualification-branch-current-authority-v1",
                vec![
                    crate::preserves_rail::string(&admission.receipt_ref),
                    crate::preserves_rail::string(&policy.policy_ref),
                    crate::preserves_rail::u64_value(at_tick),
                ],
            ))?,
            policy_current: policy.current && policy.generation == facts.policy_generation,
            capability_current: admission.decision == "pass",
            revocation_current: !revoked
                .iter()
                .any(|reference| *reference == token.issuer_ref || token.delegation_refs.contains(reference)),
            replay_current: self.activation_record(&facts.destination_branch_ref)?.is_none(),
            scope_current: facts.destination_scope.is_within(&facts.source_scope)
                && token.scope == facts.destination_scope.resource(),
            ucan_verified: self.signature_verified(&self.token, &self.signature)?,
        };
        self.admissions.push(admission.value);
        self.observations.push(current.clone());
        Ok(current)
    }
}

impl DestinationGrantPort for super::state::BranchRuntime {
    /// Mint and durably record an independent signed destination grant.
    fn realize_destination_grant(
        &mut self,
        plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
    ) -> crate::error::Result<molten_core::world_branch_authority::WorldBranchRealizationObservation> {
        let grant = crate::capability_tokens::CapabilityToken {
            token_kind: "world-branch-destination-grant".to_string(),
            context_ref: plan.destination_branch_ref.clone(),
            attenuation: "independent".to_string(),
            scope: plan.destination_scope.resource().to_string(),
            ..self.token.clone()
        };
        let grant_value = crate::capability_tokens::capability_token_value(&grant)?;
        let grant_ref = crate::preserves_rail::canonical_hash(&grant_value)?;
        let signature = self.issuer.sign(&crate::preserves_rail::canonical_bytes(&grant_value)?);
        let record = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "aspen-branch-destination-grant-v1",
            vec![
                grant_value,
                crate::preserves_rail::string(hex::encode(signature.to_bytes())),
            ],
        ))?;
        let path = crate::node_state::RelativePath::parse(&format!(
            "{}/{}.preserves",
            super::GRANT_DIRECTORY,
            crate::preserves_rail::content_ref_hex(&grant_ref)?
        ))?;
        self.publish_exact(&path, &record)?;
        let is_durable = self.root.read(&path, super::MAX_STATE_BYTES)? == record;
        let is_destination_grant_current = is_durable && self.signature_verified(&grant, &signature)?;
        let evidence_refs = plan
            .obligations
            .iter()
            .filter(|obligation| {
                **obligation != molten_core::world_branch_authority::WorldBranchObligation::DenyActivation
            })
            .map(|obligation| {
                crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                    "aspen-branch-obligation-evidence-v1",
                    vec![
                        crate::preserves_rail::string(obligation.as_str()),
                        crate::preserves_rail::string(&grant_ref),
                    ],
                ))
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        self.realized.insert(plan.plan_ref.clone(), plan.destination_branch_ref.clone());
        Ok(molten_core::world_branch_authority::WorldBranchRealizationObservation {
            plan_ref: plan.plan_ref.clone(),
            policy_ref: plan.policy_ref.clone(),
            capability_ref: plan.capability_ref.clone(),
            operation_ref: grant_ref,
            evidence_refs,
            destination_scope: plan.destination_scope.clone(),
            destination_grant_current: is_destination_grant_current,
            source_active: true,
            destination_active: false,
            transfer_generation: None,
            simulation_adapter_ref: None,
            simulation_adapter_deterministic: false,
            release_reservation_ref: None,
            promotion_admission: None,
            bearer_material_present: false,
            receipt_claims_authority: false,
        })
    }
}

/// The qualification node holds no exclusive lease, simulation adapter or
/// release reservation; those realizations fail closed.
impl LinearAuthorityTransferPort for super::state::BranchRuntime {
    fn observe_ownership(
        &mut self,
        _plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
    ) -> crate::error::Result<crate::world_branch_authority::LinearOwnershipObservation> {
        Err(super::super::invalid("qualification branch node holds no linear ownership"))
    }

    fn transfer(
        &mut self,
        _plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
        _expected_generation: u64,
        _operation_ref: &str,
    ) -> crate::error::Result<crate::world_branch_authority::LinearTransferOutcome> {
        Ok(crate::world_branch_authority::LinearTransferOutcome::Denied)
    }

    fn reconcile_transfer(
        &mut self,
        _plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
        _operation_ref: &str,
    ) -> crate::error::Result<Option<molten_core::world_branch_authority::WorldBranchRealizationObservation>> {
        Ok(None)
    }
}

impl SimulationAuthorityPort for super::state::BranchRuntime {
    fn bind_simulation(
        &mut self,
        _plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
    ) -> crate::error::Result<molten_core::world_branch_authority::WorldBranchRealizationObservation> {
        Err(super::super::invalid("qualification branch node has no deterministic simulation adapter"))
    }
}

impl PromotionReservationPort for super::state::BranchRuntime {
    fn admit_promotion_reservation(
        &mut self,
        _plan: &molten_core::world_branch_authority::WorldBranchAuthorityPlan,
    ) -> crate::error::Result<molten_core::world_branch_authority::WorldBranchPromotionReservationAdmission> {
        Err(super::super::invalid("qualification branch node holds no committed release reservation"))
    }
}

impl BranchActivationPort for super::state::BranchRuntime {
    fn activate(
        &mut self,
        decision: &molten_core::world_branch_authority::WorldBranchActivationDecision,
    ) -> crate::error::Result<crate::world_branch_authority::ActivationOutcome> {
        self.activation_calls = self
            .activation_calls
            .checked_add(1)
            .ok_or_else(|| super::super::invalid("activation count overflowed"))?;
        let destination = self
            .realized
            .get(&decision.plan_ref)
            .cloned()
            .ok_or_else(|| super::super::invalid("activation decision names an unrealized plan"))?;
        let path = Self::activation_path(&destination)?;
        if self.root.try_exists(&path)? {
            return Ok(crate::world_branch_authority::ActivationOutcome::Denied);
        }
        let record = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "aspen-branch-activation-v1",
            vec![
                crate::preserves_rail::string(&decision.decision_ref),
                crate::preserves_rail::string(&decision.plan_ref),
                crate::preserves_rail::string(&destination),
            ],
        ))?;
        self.root.write_new(&path, &record)?;
        Ok(crate::world_branch_authority::ActivationOutcome::Activated)
    }

    fn reconcile_activation(
        &mut self,
        decision: &molten_core::world_branch_authority::WorldBranchActivationDecision,
    ) -> crate::error::Result<crate::world_branch_authority::ActivationOutcome> {
        let destination = self
            .realized
            .get(&decision.plan_ref)
            .cloned()
            .ok_or_else(|| super::super::invalid("activation decision names an unrealized plan"))?;
        Ok(if self.activation_record(&destination)?.as_deref() == Some(decision.decision_ref.as_str()) {
            crate::world_branch_authority::ActivationOutcome::Activated
        } else {
            crate::world_branch_authority::ActivationOutcome::Denied
        })
    }
}

impl BranchAuthorityReceiptPort for super::state::BranchRuntime {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_bytes: &[u8]) -> crate::error::Result<()> {
        let value = crate::preserves_rail::parse_canonical_bytes(canonical_bytes)?;
        let path = crate::node_state::RelativePath::parse(&format!(
            "{}/{}.preserves",
            super::RECEIPT_DIRECTORY,
            crate::preserves_rail::content_ref_hex(receipt_ref)?
        ))?;
        self.publish_exact(&path, canonical_bytes)?;
        self.receipts.push(value);
        Ok(())
    }
}
