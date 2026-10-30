mod receipt;

/// All request facts are durably bound before external dispatch. This contains
/// public claims and decision references only, never signing material.
pub struct Stage {
    pub operation_ref: String,
    pub bytes: Vec<u8>,
    pub claim_bytes: Vec<u8>,
    pub staged: molten_core::world_witness::StagedClaim,
    pub authentication_ref: String,
    pub authority_ref: String,
    pub witness_policy: molten_core::world_witness::WitnessPolicy,
}

impl Stage {
    pub fn prepare(input: &crate::world_witness::Transition<'_>) -> crate::error::Result<Self> {
        if input.claim.claim_ref != input.plan.claim_ref
            || input.claim.claim.branch_id != input.plan.after.branch_id
            || input.claim.claim.successor_generation != input.plan.after.generation
            || input.claim.claim.successor_head != input.plan.after.head
            || input.claim.claim.policy_ref != input.plan.after.policy_ref
            || input.claim.claim.branch_class != input.plan.after.branch_class
            || input.claim.claim.expected_generation != input.plan.before.as_ref().map_or(0, |state| state.generation)
            || input.claim.claim.expected_head.as_ref() != input.plan.before.as_ref().map(|state| &state.head)
        {
            return Err(crate::error::Failure::invalid_harness("witness transition substitutes claim facts"));
        }
        let canonical = crate::world_head::canonical_world_head_claim(&input.claim.claim)?;
        if canonical.bytes != input.claim.bytes || canonical.claim_ref != input.claim.claim_ref {
            return Err(crate::error::Failure::invalid_harness("witness requires actual canonical claim bytes"));
        }
        let (_, after) = crate::world_head::canonical_world_head_state(&input.plan.after)?;
        let before = input.plan.before.as_ref().map(crate::world_head::canonical_world_head_state).transpose()?;
        let before_ref = before.as_ref().map_or("none", |(reference, _)| reference.as_str());
        let retained_ref = match &input.currentness.retained {
            molten_core::world_witness::RetainedWitnessState::Empty => "explicit-initial-enrollment".to_owned(),
            molten_core::world_witness::RetainedWitnessState::Missing => {
                return Err(crate::error::Failure::invalid_harness("missing retained witness state"));
            }
            molten_core::world_witness::RetainedWitnessState::Present(highest) => {
                crate::preserves_rail::content_ref_from_bytes(&crate::world_witness::records::canonical_highest(
                    highest,
                )?)
            }
        };
        let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "molten-world-witness-stage-v1",
            vec![
                crate::preserves_rail::string(input.claim.claim_ref.as_str()),
                crate::preserves_rail::string(before_ref),
                crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&after)),
                crate::preserves_rail::string(input.authentication_ref),
                crate::preserves_rail::string(input.authority_ref),
                crate::preserves_rail::string(input.currentness.selected_profile.as_str()),
                crate::preserves_rail::string(&input.plan.choregraph_after_identity),
                crate::preserves_rail::string(input.plan.choregraph_before_identity.as_deref().unwrap_or("none")),
                crate::preserves_rail::string(retained_ref),
                crate::preserves_rail::string(policy_identity(&input.currentness.policy)?),
                crate::preserves_rail::string(input.plan.currentness.as_str()),
            ],
        ))?;
        let operation_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        Ok(Self {
            staged: molten_core::world_witness::StagedClaim {
                operation_id: operation_ref.clone(),
                transition: input.plan.clone(),
                selected_profile: input.currentness.selected_profile,
                retained: input.currentness.retained.clone(),
            },
            operation_ref,
            bytes,
            claim_bytes: canonical.bytes,
            authentication_ref: input.authentication_ref.to_owned(),
            authority_ref: input.authority_ref.to_owned(),
            witness_policy: input.currentness.policy.clone(),
        })
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        let claim = crate::world_head::parse_canonical_world_head_claim(&self.claim_bytes)?;
        let rebuilt = Self::prepare(&crate::world_witness::Transition {
            claim: &claim,
            plan: &self.staged.transition,
            authentication_ref: &self.authentication_ref,
            authority_ref: &self.authority_ref,
            currentness: molten_core::world_witness::CurrentnessInput {
                selected_profile: self.staged.selected_profile,
                policy: self.witness_policy.clone(),
                operation_id: self.operation_ref.clone(),
                subject: molten_core::world_witness::subject(&self.staged),
                local_generation: self.staged.transition.before.as_ref().map_or(0, |state| state.generation),
                retained: self.staged.retained.clone(),
                observation: None,
                now: 0,
            },
        })?;
        if rebuilt.bytes != self.bytes || rebuilt.operation_ref != self.operation_ref || rebuilt.staged != self.staged {
            return Err(crate::error::Failure::invalid_harness(
                "staged fields do not match canonical durable operation",
            ));
        }
        Ok(())
    }
}

pub fn policy_identity(policy: &molten_core::world_witness::WitnessPolicy) -> crate::error::Result<String> {
    let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
        "molten-world-witness-policy-v1",
        vec![
            crate::preserves_rail::string(policy.required_profile.as_str()),
            crate::preserves_rail::string(&policy.provider.provider_id),
            crate::preserves_rail::string(&policy.provider.contract_id),
            crate::preserves_rail::string(&policy.provider.source_revision),
            crate::preserves_rail::string(&policy.provider.license),
            crate::preserves_rail::sequence(
                policy.allowed_witnesses.iter().map(crate::preserves_rail::string).collect(),
            ),
            crate::preserves_rail::u64_value(
                u64::try_from(policy.minimum_quorum).map_err(crate::world_witness::store::failure)?,
            ),
            crate::preserves_rail::u64_value(policy.maximum_age),
        ],
    ))?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&bytes))
}
