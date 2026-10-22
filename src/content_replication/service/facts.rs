// The observed facts of a reconciliation.
//
// The facts are the authority, identity, membership, placement, and time
// observations the shell reads before it plans. The evidence references bind
// the observations the plan and the receipt carry.

use molten_core::content_replication::*;

use super::*;

pub(super) struct CurrentFacts {
    pub(super) authority: AuthorityObservation,
    pub(super) identity: IdentityObservation,
    pub(super) membership: MembershipObservation,
    pub(super) placement: PlacementObservation,
    pub(super) time: TimeObservation,
}

impl CurrentFacts {
    pub(super) fn evidence_refs(&self) -> Vec<String> {
        vec![
            self.authority.observation_ref.clone(),
            self.identity.observation_ref.clone(),
            self.membership.observation_ref.clone(),
            self.placement.observation_ref.clone(),
            self.time.observation_ref.clone(),
        ]
    }
}

pub(super) struct FactPorts<'a> {
    pub(super) authority_port: &'a mut dyn AuthorityPort,
    pub(super) identity_port: &'a mut dyn IdentityPort,
    pub(super) membership_port: &'a mut dyn MembershipPort,
    pub(super) placement_port: &'a mut dyn PlacementPort,
    pub(super) time_port: &'a mut dyn TimePort,
}

pub(super) fn observe_current_facts(manifest: &Manifest, input: FactPorts<'_>) -> crate::error::Result<CurrentFacts> {
    let FactPorts {
        authority_port,
        identity_port,
        membership_port,
        placement_port,
        time_port,
    } = input;
    let authority = authority_port.observe(manifest)?;
    validate_authority(manifest, &authority)?;
    let identity = identity_port.observe(manifest)?;
    validate_identity(manifest, &identity)?;
    let membership = membership_port.observe(manifest)?;
    validate_membership(manifest, &membership)?;
    let placement = placement_port.observe(manifest)?;
    validate_placement(manifest, &placement)?;
    let time = time_port.observe(manifest)?;
    validate_ref(&time.observation_ref, "replication time observation")?;
    Ok(CurrentFacts {
        authority,
        identity,
        membership,
        placement,
        time,
    })
}
