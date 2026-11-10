//! Operator scope trust publication: the trust position must match the linearizable read.

/// Publish only the holder credential; trust is frozen after the final real observation.
pub(super) fn publish(
    node: &super::Node,
    fixture: &crate::producer::Node,
    intent: &super::operations::Intent,
    effect_ref: &str,
) -> crate::producer::Result<()> {
    let mut window = crate::authority::Window::around_now()?;
    window.not_before = window.not_before.min(intent.observation.unix_seconds);
    window.expires = window.expires.max(intent.observation.unix_seconds.checked_add(1).ok_or("time overflow")?);
    let credential = node.issuer.scope_credential(
        &intent.prepared.request,
        window,
        nonce(node.index, intent.observation.currentness.logical_now)?,
    )?;
    crate::authority::scope::write_credential(&fixture.root, effect_ref, &intent.observation.read, &credential)?;
    Ok(())
}

/// Only diagnostic reads may be retried. An observation is data, never mutation authority.
pub(super) fn select(
    nodes: &[super::Node],
    cohort: &crate::producer::Cohort,
    eligible: &[usize],
) -> crate::producer::Result<(usize, u64, molten::collaboration_scope::native_runtime::Observation)> {
    let mut last = String::from("no current replica");
    let mut tick = 3;
    let mut rounds = 0_u32;
    let started = super::setup::now()?;
    loop {
        rounds += 1;
        for &index in eligible {
            let node = nodes.get(index).ok_or("metadata candidate missing")?;
            let fixture = cohort.nodes.get(index).ok_or("metadata fixture missing")?;
            tick += 1;
            let first = match observe(node, fixture, tick)? {
                Ok(value) => value,
                Err(error) => {
                    last = error;
                    continue;
                }
            };
            node.issuer.provision_scope(&fixture.root, &crate::authority::scope::TrustInput {
                policies: &[super::setup::SCOPE_POLICY],
                revoked_tokens: &[],
                logical_position: first.currentness.committed_index,
                unix_seconds: super::setup::now()?,
            })?;
            tick += 1;
            match observe(node, fixture, tick)? {
                Ok(final_read)
                    if final_read.currentness.committed_index == first.currentness.committed_index
                        && final_read.currentness.logical_now == first.currentness.committed_index =>
                {
                    return Ok((index, tick, final_read));
                }
                Ok(_) => last = "applied position moved during trust alignment".into(),
                Err(error) => last = error,
            }
        }
        if super::setup::now()?.saturating_sub(started) >= 1800 {
            return Err(format!(
                "no stable real quorum observation within bound after {rounds} metadata rounds: {last}"
            )
            .into());
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn observe(
    node: &super::Node,
    fixture: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<Result<molten::collaboration_scope::native_runtime::Observation, String>> {
    let read = crate::producer::reference(&format!("collaboration-native-read-{}-{tick}", node.index));
    let operation = super::operations::wire::operation("scope-observe", vec![
        molten::preserves_rail::string(crate::producer::SERVICE),
        molten::preserves_rail::string(&read),
    ]);
    let request = super::flow::authorize(&super::process::host_of(node), fixture, &operation, tick)?;
    let (success, output) = super::process::invoke(&super::process::host_of(node), "scope-observe", tick, None)?;
    if !success {
        return Ok(Err(output));
    }
    let receipt = molten::preserves_rail::parse_text(super::flow::value(&output, "node-service-host-receipt-v1")?)?;
    let receipt = molten::preserves_rail::simple_record_fields(&receipt, "node-service-host-receipt-v1", 5)?;
    if super::operations::wire::text(&receipt[0])? != "scope-observe"
        || super::operations::wire::text(&receipt[1])? != fixture.bundle_ref
        || super::operations::wire::number(&receipt[2])? != tick
        || super::operations::wire::text(&receipt[3])?
            != molten::node_service_host::authority::request_ref(&fixture.bundle_ref, &request)?
    {
        return Err("scope observation receipt request mismatch".into());
    }
    let value = molten::preserves_rail::value_to_iovalue(&receipt[4]);
    let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-scope-observed-v1", 5)?;
    if super::operations::wire::text(&fields[0])? != crate::producer::SERVICE
        || super::operations::wire::text(&fields[1])? != read
        || super::operations::wire::text(&fields[2])?
            != molten::node_service_host::effects::config::reference(&fixture.effects)?
    {
        return Err("scope observation configuration or identity mismatch".into());
    }
    molten::preserves_rail::validate_content_ref(&super::operations::wire::text(&fields[3])?)?;
    let observation = molten::collaboration_scope::native_runtime::Observation::parse(
        &molten::preserves_rail::value_to_iovalue(&fields[4]),
    )?;
    if observation.read.as_str() != read {
        return Err("scope observation read mismatch".into());
    }
    Ok(Ok(observation))
}

/// The replicated command refers to these actual canonical bytes on every application replica.
pub(super) fn publish_command(
    fixture: &crate::producer::Node,
    intent: &super::operations::Intent,
) -> crate::producer::Result<()> {
    let bytes =
        molten::preserves_rail::canonical_bytes(&molten::collaboration_scope::intent_value(&intent.prepared.request))?;
    let value = molten::system_extension::NativeCallbackValue {
        value_ref: molten::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    };
    let namespace = fixture
        .root
        .ledger()?
        .open_subdir(&molten::node_state::RelativePath::parse(&fixture.runtime.callback_namespace)?)?;
    let mut values = molten::system_extension::CapabilityCallbackValuePort::new(namespace)?;
    molten::system_extension::NativeCallbackValuePort::publish(
        &mut values,
        &value,
        molten::collaboration_scope::MAX_RECORD_BYTES,
    )
    .map_err(|error| format!("proposal value publication failed: {error:?}"))?;
    Ok(())
}

fn nonce(index: u64, marker: u64) -> crate::producer::Result<[u8; 32]> {
    // Each published trust attempt carries its own holder nonce; a repeated nonce would make the
    // credential bytes identical and a real quorum read could refuse it as replay.
    let mut nonce = [0u8; 32];
    nonce[..8].copy_from_slice(&marker.to_le_bytes());
    nonce[31] = u8::try_from(index).map_err(|_| "cohort position exceeds one byte")?;
    Ok(nonce)
}

/// Reopen the owning application's real store after its serving process has closed.
pub(super) fn persisted(
    fixture: &crate::producer::Node,
) -> crate::producer::Result<molten::collaboration_scope::State> {
    let input = &fixture.runtime.scope_store;
    let profile = input.profile.clone().into_profile()?;
    let namespace = input.namespace.clone().into_descriptor(&profile)?;
    let root = molten::local_store::DurableStoreRoot::from_ledger(
        &fixture.root.ledger_store()?,
        &molten::local_store::Locator::parse(&input.storage_namespace)?,
    )?;
    let adapter = molten::fabric_durability::RedbDurableStateAdapter::open_root(root, profile, namespace)?;
    let domain = adapter.state().descriptor.atomicity_domain.clone();
    molten::collaboration_scope::Database::new(adapter, domain)?
        .load()?
        .ok_or_else(|| "actual application state was not durably retained".into())
}
