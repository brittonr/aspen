const MAX_CHECKPOINT_BYTES: u64 = 32 * 1024 * 1024;

pub(super) fn save(
    service: &super::Service,
    sequence: u64,
    session: &molten_core::federation::Session,
    scope: &crate::node_state::Scope,
) -> crate::error::Result<()> {
    let is_terminal = matches!(
        session.phase,
        molten_core::federation::SessionPhase::Committed | molten_core::federation::SessionPhase::Denied
    );
    let active_count = service.active.len() - usize::from(service.active.contains_key(&session.session_ref))
        + usize::from(!is_terminal);
    if active_count > service.manifest.bounds().concurrent_sessions {
        return Err(invalid("active checkpoint exceeds session bound"));
    }
    let before = service.active.values().filter(|prior| prior.session_ref < session.session_ref);
    let after = service.active.values().filter(|prior| prior.session_ref > session.session_ref);
    let sessions = before.chain((!is_terminal).then_some(session)).chain(after);
    let value = crate::preserves_rail::record("federation-recovery-v1", vec![
        crate::preserves_rail::string(service.manifest.manifest_ref()),
        crate::preserves_rail::u64_value(service.host.state().generation),
        crate::preserves_rail::u64_value(sequence),
        crate::preserves_rail::sequence(sessions.map(super::checkpoint::encode).collect()),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if u64::try_from(bytes.len()).map_err(|_| invalid("checkpoint byte overflow"))? > MAX_CHECKPOINT_BYTES {
        return Err(invalid("checkpoint exceeds retained byte bound"));
    }
    scope.write(&path()?, &bytes)
}

pub(super) fn restore(service: &mut super::Service, scope: &crate::node_state::Scope) -> crate::error::Result<()> {
    if !scope.try_exists(&path()?)? {
        return Ok(());
    }
    let value = crate::preserves_rail::parse_canonical_bytes(&scope.read(&path()?, MAX_CHECKPOINT_BYTES)?)?;
    let fields = value
        .collect_simple_record("federation-recovery-v1", Some(4))
        .ok_or_else(|| invalid("malformed federation checkpoint"))?;
    if super::checkpoint::string(&fields[0])? != service.manifest.manifest_ref()
        || super::checkpoint::integer(&fields[1])? != service.host.state().generation
    {
        return Err(invalid("checkpoint manifest or generation is fenced"));
    }
    let mut active = std::collections::BTreeMap::new();
    for value in super::checkpoint::items(&fields[3], service.manifest.bounds().concurrent_sessions)? {
        let session =
            super::checkpoint::decode(&crate::preserves_rail::value_to_iovalue(&value), service.manifest.bounds())?;
        if session.generation != service.host.state().generation
            || matches!(
                session.phase,
                molten_core::federation::SessionPhase::Committed | molten_core::federation::SessionPhase::Denied
            )
            || !service
                .manifest
                .peers()
                .iter()
                .any(|peer| peer.peer_ref == session.plan.peer_ref && peer.signer_ref == session.plan.signer_ref)
            || active.insert(session.session_ref.clone(), session).is_some()
        {
            return Err(invalid("checkpoint session is fenced, terminal, duplicate or not configured"));
        }
    }
    service.status_sequence = super::checkpoint::integer(&fields[2])?;
    service.active = active;
    Ok(())
}
fn path() -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse("federation-recovery.preserves")
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
