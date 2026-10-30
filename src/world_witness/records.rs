pub mod receipt;
mod wire;

#[cfg(test)]
mod tests;

pub use receipt::PersistenceDecision;
pub use receipt::ReconciliationDecision;
pub use receipt::RoleDecision;
pub use receipt::required_non_claims;

const HIGHEST_RECORD: &str = "molten-world-highest-witness-v1";

/// Detached observations: identities are supplied by their actual owning decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub operation_id: String,
    pub claim_ref: molten_core::world_head::WorldHeadClaimRef,
    pub authentication_ref: molten_core::world_head::WorldHeadAuthenticationDecisionRef,
    pub authority_ref: molten_core::world_head::WorldHeadAuthorityRef,
    pub profile: molten_core::world_witness::AssuranceProfile,
    pub authentication: receipt::RoleDecision,
    pub branch_authorization: receipt::RoleDecision,
    pub witness_currentness: receipt::RoleDecision,
    pub local_persistence: receipt::PersistenceDecision,
    pub reconciliation: receipt::ReconciliationDecision,
    pub highest: Option<molten_core::world_witness::HighestWitnessState>,
    pub witness_grants_authority: bool,
    pub non_claims: Vec<String>,
}

/// Canonical durable projection, not a new admission or signature verification.
// r[impl molten.world_witness.currentness]
pub fn canonical_highest(state: &molten_core::world_witness::HighestWitnessState) -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&highest_value(state))
}

pub fn parse_highest(bytes: &[u8]) -> crate::error::Result<molten_core::world_witness::HighestWitnessState> {
    let value = crate::preserves_rail::strict_canonical_decode(bytes)?.value;
    parse_highest_value(&value)
}

fn highest_value(state: &molten_core::world_witness::HighestWitnessState) -> preserves::IOValue {
    crate::preserves_rail::record(HIGHEST_RECORD, vec![
        crate::preserves_rail::string(state.subject.branch_id.as_str()),
        crate::preserves_rail::string(state.subject.claim_ref.as_str()),
        crate::preserves_rail::u64_value(state.subject.generation),
        crate::preserves_rail::string(&state.checkpoint.provider_id),
        crate::preserves_rail::string(&state.checkpoint.checkpoint_id),
        crate::preserves_rail::u64_value(state.checkpoint.revision),
        crate::preserves_rail::bool_value(state.checkpoint.signature_verified),
        crate::preserves_rail::u64_value(state.checkpoint.observed_at),
        wire::optional_text(state.consistency_predecessor.as_deref()),
        crate::preserves_rail::sequence(state.quorum.iter().map(crate::preserves_rail::string).collect()),
    ])
}

fn parse_highest_value(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<molten_core::world_witness::HighestWitnessState> {
    let fields = wire::fields(value, HIGHEST_RECORD, 10)?;
    let branch_id = molten_core::world_head::WorldBranchId::new(wire::text(&fields[0])?)
        .map_err(|issue| wire::invalid(issue.to_string()))?;
    let claim_ref = molten_core::world_head::WorldHeadClaimRef::new(wire::text(&fields[1])?)
        .map_err(|issue| wire::invalid(issue.to_string()))?;
    let quorum_rows = wire::texts(&fields[9])?;
    let quorum: std::collections::BTreeSet<_> = quorum_rows.iter().cloned().collect();
    if quorum.len() != quorum_rows.len() || !quorum_rows.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(wire::invalid("noncanonical witness quorum"));
    }
    Ok(molten_core::world_witness::HighestWitnessState {
        subject: molten_core::world_witness::WitnessSubject {
            branch_id,
            claim_ref,
            generation: wire::number(&fields[2])?,
        },
        checkpoint: molten_core::world_witness::SignedCheckpoint {
            provider_id: wire::text(&fields[3])?,
            checkpoint_id: wire::text(&fields[4])?,
            revision: wire::number(&fields[5])?,
            signature_verified: wire::boolean(&fields[6])?,
            observed_at: wire::number(&fields[7])?,
        },
        consistency_predecessor: wire::parse_optional_text(&fields[8])?,
        quorum,
    })
}
