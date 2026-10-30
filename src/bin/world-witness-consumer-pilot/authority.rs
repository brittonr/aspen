pub fn initialize(root: &std::path::Path, trust: &molten::world_witness::authority::wire::Trust) -> super::Result<()> {
    let replay = durable_authority_state_redb::ledger::ReplayRedbStore::open(
        root.join("replay.redb"),
        durable_authority_state_redb::SyncMode::Immediate,
    )
    .map_err(|error| format!("durable authority store open: {error:?}"))?;
    let revocations = durable_authority_state_redb::view::RevocationRedbStore::open(
        root.join("revocation.redb"),
        durable_authority_state_redb::SyncMode::Immediate,
    )
    .map_err(|error| format!("durable authority store open: {error:?}"))?;
    molten::world_witness::authority::provision_revocations(&revocations, trust)?;
    drop(replay);
    Ok(())
}

pub fn open(
    root: &std::path::Path,
) -> super::Result<molten::world_witness::authority::Current<molten::fabric_time::LiveClockAdapter>> {
    let operator = molten::node_state::DirectoryRoot::open_existing(root)?;
    let replay = durable_authority_state_redb::ledger::ReplayRedbStore::open(
        root.join("replay.redb"),
        durable_authority_state_redb::SyncMode::Immediate,
    )
    .map_err(|error| format!("durable authority store open: {error:?}"))?;
    let revocations = durable_authority_state_redb::view::RevocationRedbStore::open(
        root.join("revocation.redb"),
        durable_authority_state_redb::SyncMode::Immediate,
    )
    .map_err(|error| format!("durable authority store open: {error:?}"))?;
    Ok(molten::world_witness::authority::Current::open(
        &operator,
        replay,
        revocations,
        molten::world_witness::live_clock()?,
    )?)
}
