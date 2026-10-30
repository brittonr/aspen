/// Explicit operator provisioning, never called automatically during admission.
pub fn provision_revocations(
    store: &durable_authority_state_redb::view::RevocationRedbStore,
    trust: &super::wire::Trust,
) -> crate::error::Result<()> {
    let snapshot = snapshot(trust)?;
    let observed =
        revocation_view_core::RevocationViewPort::load(store, snapshot.source()).map_err(|_| super::denied())?;
    let command = match revocation_view_core::plan_update(snapshot, observed) {
        revocation_view_core::UpdateDecision::Install(command)
        | revocation_view_core::UpdateDecision::Advance(command) => command,
        revocation_view_core::UpdateDecision::ExactDuplicate => return Ok(()),
        revocation_view_core::UpdateDecision::StaleGeneration
        | revocation_view_core::UpdateDecision::GenerationConflict
        | revocation_view_core::UpdateDecision::StateUnavailable
        | revocation_view_core::UpdateDecision::VersionExhausted => return Err(super::denied()),
    };
    match revocation_view_core::RevocationViewPort::compare_and_swap(store, command).map_err(|_| super::denied())? {
        revocation_view_core::CommitObservation::Applied {
            durability: revocation_view_core::DurabilityObservation::Synchronized,
            ..
        } => Ok(()),
        revocation_view_core::CommitObservation::Applied {
            durability:
                revocation_view_core::DurabilityObservation::CommitOnly
                | revocation_view_core::DurabilityObservation::Unknown,
            ..
        }
        | revocation_view_core::CommitObservation::Stale { .. }
        | revocation_view_core::CommitObservation::OutcomeUnknown => Err(super::denied()),
    }
}

pub(super) fn current<'a>(
    store: &'a durable_authority_state_redb::view::RevocationRedbStore,
    trust: &super::wire::Trust,
    now: u64,
) -> crate::error::Result<
    basalt::ucan::DurableRevocationChecker<'a, durable_authority_state_redb::view::RevocationRedbStore>,
> {
    let expected = snapshot(trust)?;
    let observed =
        revocation_view_core::RevocationViewPort::load(store, expected.source()).map_err(|_| super::denied())?;
    let revocation_view_core::SnapshotObservation::Present(stored) = observed else {
        return Err(super::denied());
    };
    if stored.snapshot().identity() != expected.identity() {
        return Err(super::denied());
    }
    let context = basalt::ucan::DurableRevocationContext::new(basalt::ucan::DurableRevocationContextInput {
        source: *expected.source().as_bytes(),
        minimum_generation: trust.authority_generation,
        current_time_seconds: now,
    })
    .map_err(|_| super::denied())?;
    Ok(basalt::ucan::DurableRevocationChecker::new(store, context))
}

fn snapshot(trust: &super::wire::Trust) -> crate::error::Result<revocation_view_core::RevocationSnapshot> {
    let source = crate::preserves_rail::record("world-head-revocation-source-v1", vec![
        crate::preserves_rail::string(&trust.issuer),
        preserves::IOValue::bytes(trust.issuer_key),
    ]);
    let source = *blake3::hash(&crate::preserves_rail::canonical_bytes(&source)?).as_bytes();
    let entries = trust
        .revoked
        .iter()
        .map(|reference| {
            let reference = basalt::ucan::ProofReference::new(reference.to_vec());
            revocation_view_core::RevocationEntryDigest::new(basalt::ucan::durable_revocation_entry_digest(&reference))
                .map_err(|_| super::denied())
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    revocation_view_core::RevocationSnapshot::admit(revocation_view_core::SnapshotInput {
        source: Some(revocation_view_core::SourceDigest::new(source).map_err(|_| super::denied())?),
        generation: Some(
            revocation_view_core::SourceGeneration::new(trust.authority_generation).map_err(|_| super::denied())?,
        ),
        observed_at: revocation_view_core::UnixTime::from_seconds(trust.not_before),
        effective_from: revocation_view_core::UnixTime::from_seconds(trust.not_before),
        expires_at: revocation_view_core::UnixTime::from_seconds(trust.expires),
        completeness: revocation_view_core::CompletenessObservation::Complete,
        // Authenticated by the independently installed operator capability, never by the presented token.
        authenticity: revocation_view_core::AuthenticityObservation::Verified,
        entries,
        entry_limit: revocation_view_core::EntryLimit::new(4096).map_err(|_| super::denied())?,
    })
    .map_err(|_| super::denied())
}
