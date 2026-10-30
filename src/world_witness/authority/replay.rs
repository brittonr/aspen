pub(super) struct Guard<'a> {
    pub store: &'a durable_authority_state_redb::ledger::ReplayRedbStore,
    pub trust: &'a super::wire::Trust,
    pub claim_ref: &'a str,
}

impl basalt::ucan::HolderInvocationReplayAdmission for Guard<'_> {
    fn admit_holder_invocation(
        &self,
        candidate: &basalt::ucan::HolderInvocationReplayCandidate<'_>,
    ) -> Result<(), basalt::ucan::HolderInvocationReplayError> {
        self.admit(candidate).map_err(|_| basalt::ucan::HolderInvocationReplayError::Backend)
    }
}

impl Guard<'_> {
    fn admit(&self, candidate: &basalt::ucan::HolderInvocationReplayCandidate<'_>) -> crate::error::Result<()> {
        let scope = replay_ledger_core::ReplayScope::new(replay_ledger_core::ReplayScopeInput {
            tenant: replay_ledger_core::TenantDigest::new(digest(
                "world-head-tenant-v1",
                self.trust.issuer.as_bytes(),
            )?)
            .map_err(|_| super::denied())?,
            actor: replay_ledger_core::ActorDigest::new(digest(
                "world-head-actor-v1",
                candidate.audience().as_str().as_bytes(),
            )?)
            .map_err(|_| super::denied())?,
            operation_profile: replay_ledger_core::OperationProfileDigest::new(digest(
                "world-head-profile-v1",
                b"mutate",
            )?)
            .map_err(|_| super::denied())?,
            request: replay_ledger_core::RequestDigest::new(*candidate.request_digest().as_bytes())
                .map_err(|_| super::denied())?,
            nonce: replay_ledger_core::NonceDigest::new(digest("world-head-nonce-v1", candidate.nonce().as_bytes())?)
                .map_err(|_| super::denied())?,
            generation: replay_ledger_core::AuthorityGeneration::new(self.trust.authority_generation)
                .map_err(|_| super::denied())?,
            namespace: replay_ledger_core::LedgerNamespaceDigest::new(digest(
                "world-head-ledger-v1",
                b"world-head-witness",
            )?)
            .map_err(|_| super::denied())?,
        });
        let attempt =
            replay_ledger_core::AttemptIdentity::new(digest("world-head-attempt-v1", self.claim_ref.as_bytes())?)
                .map_err(|_| super::denied())?;
        let observed = replay_ledger_core::ReplayLedgerPort::load(self.store, scope.slot_identity())
            .map_err(|_| super::denied())?;
        match replay_ledger_core::plan_consumption(scope, attempt, observed) {
            replay_ledger_core::ConsumptionDecision::Reserved { attempt: owner } if owner == attempt => Ok(()),
            replay_ledger_core::ConsumptionDecision::First { command } => {
                let committed = replay_ledger_core::ReplayLedgerPort::compare_and_swap(self.store, command)
                    .map_err(|_| super::denied())?;
                match committed {
                    replay_ledger_core::CommitObservation::Applied {
                        durability: replay_ledger_core::DurabilityObservation::Synchronized,
                        ..
                    } => Ok(()),
                    replay_ledger_core::CommitObservation::Stale { .. } => self.exact_reserved(scope, attempt),
                    replay_ledger_core::CommitObservation::Applied {
                        durability:
                            replay_ledger_core::DurabilityObservation::CommitOnly
                            | replay_ledger_core::DurabilityObservation::Unknown,
                        ..
                    }
                    | replay_ledger_core::CommitObservation::OutcomeUnknown => Err(super::denied()),
                }
            }
            replay_ledger_core::ConsumptionDecision::Reserved { .. }
            | replay_ledger_core::ConsumptionDecision::ExactDuplicate { .. }
            | replay_ledger_core::ConsumptionDecision::Cancelled
            | replay_ledger_core::ConsumptionDecision::Uncertain
            | replay_ledger_core::ConsumptionDecision::Conflict
            | replay_ledger_core::ConsumptionDecision::StateUnavailable => Err(super::denied()),
        }
    }

    fn exact_reserved(
        &self,
        scope: replay_ledger_core::ReplayScope,
        attempt: replay_ledger_core::AttemptIdentity,
    ) -> crate::error::Result<()> {
        let observed = replay_ledger_core::ReplayLedgerPort::load(self.store, scope.slot_identity())
            .map_err(|_| super::denied())?;
        match replay_ledger_core::plan_consumption(scope, attempt, observed) {
            replay_ledger_core::ConsumptionDecision::Reserved { attempt: owner } if owner == attempt => Ok(()),
            replay_ledger_core::ConsumptionDecision::Reserved { .. }
            | replay_ledger_core::ConsumptionDecision::First { .. }
            | replay_ledger_core::ConsumptionDecision::ExactDuplicate { .. }
            | replay_ledger_core::ConsumptionDecision::Cancelled
            | replay_ledger_core::ConsumptionDecision::Uncertain
            | replay_ledger_core::ConsumptionDecision::Conflict
            | replay_ledger_core::ConsumptionDecision::StateUnavailable => Err(super::denied()),
        }
    }
}

fn digest(label: &'static str, bytes: &[u8]) -> crate::error::Result<[u8; 32]> {
    let value = crate::preserves_rail::record(label, vec![preserves::IOValue::bytes(bytes.to_vec())]);
    Ok(*blake3::hash(&crate::preserves_rail::canonical_bytes(&value)?).as_bytes())
}
