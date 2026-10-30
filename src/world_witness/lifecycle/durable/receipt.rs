impl super::Stage {
    pub(crate) fn receipt(
        &self,
        highest: Option<molten_core::world_witness::HighestWitnessState>,
        witness: crate::world_witness::records::RoleDecision,
    ) -> crate::error::Result<crate::world_witness::records::Receipt> {
        Ok(crate::world_witness::records::Receipt {
            operation_id: self.operation_ref.clone(),
            claim_ref: self.staged.transition.claim_ref.clone(),
            authentication_ref: molten_core::world_head::WorldHeadAuthenticationDecisionRef::new(
                &self.authentication_ref,
            )
            .map_err(crate::world_witness::store::failure)?,
            authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(&self.authority_ref)
                .map_err(crate::world_witness::store::failure)?,
            profile: self.staged.selected_profile,
            authentication: crate::world_witness::records::RoleDecision::Admitted,
            branch_authorization: crate::world_witness::records::RoleDecision::Admitted,
            witness_currentness: witness,
            local_persistence: crate::world_witness::records::PersistenceDecision::Committed,
            reconciliation: crate::world_witness::records::ReconciliationDecision::NotRequired,
            highest,
            witness_grants_authority: false,
            non_claims: crate::world_witness::records::required_non_claims(),
        })
    }
}
