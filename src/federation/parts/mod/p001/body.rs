
pub fn announce_resource(input: &AnnounceResourceInput<'_>) -> Result<Announcement> {
    validate_peer(input.peer)?;
    validate_resource(input.resource)?;
    validate_refs(input.policy_refs, "federation announcement policy ref")?;
    let payload = announcement_payload_value(input.peer, input.resource, input.policy_refs);
    let signature = signature_record(&payload, input.signer, ANNOUNCEMENT_PURPOSE, input.trust_root, input.key)?;
    let value = record("federation-announcement-v1", vec![
        string(ANNOUNCEMENT_SCHEMA),
        record("payload", vec![payload]),
        signature,
        record("checks", vec![sequence(vec![
            record("check", vec![string("signed-announcement"), string("pass")]),
            record("check", vec![string("resource-ref-binding"), string("pass")]),
            record("check", vec![string("announcement-is-a-hint"), string("pass")]),
        ])]),
    ]);
    parse_announcement(&value, input.trust_root, input.key)
}

pub fn inventory_ledger(root: &Path, peer: &str, signer: &str, trust_root: &str, key: &str) -> Result<Inventory> {
    validate_peer(peer)?;
    let resources = ledger::list_artifacts(root)?
        .into_iter()
        .map(|entry| {
            Resource::new(entry.artifact_kind, entry.artifact_ref, "molten.ledger.artifact.v1", "ledger-local", peer)
        })
        .collect::<Vec<_>>();
    inventory_for_resources(peer, &resources, signer, trust_root, key)
}

pub fn inventory_for_resources(
    peer: &str,
    resources: &[Resource],
    signer: &str,
    trust_root: &str,
    key: &str,
) -> Result<Inventory> {
    inventory_for_resources_with_delegates(&InventoryWithDelegatesInput {
        peer,
        resources,
        delegates: &[],
        signer,
        trust_root,
        key,
    })
}

pub fn inventory_for_resources_with_delegates(input: &InventoryWithDelegatesInput<'_>) -> Result<Inventory> {
    validate_peer(input.peer)?;
    for resource in input.resources {
        validate_resource(resource)?;
    }
    for delegate in input.delegates {
        require_ref(&delegate.resource_ref, "federation delegate resource ref")?;
    }
    let payload = inventory_payload_value(input.peer, input.resources, input.delegates);
    let signature = signature_record(&payload, input.signer, INVENTORY_PURPOSE, input.trust_root, input.key)?;
    let value = record("federation-inventory-v1", vec![
        string(INVENTORY_SCHEMA),
        record("payload", vec![payload]),
        signature,
        record("checks", vec![sequence(vec![
            record("check", vec![string("signed-inventory"), string("pass")]),
            record("check", vec![string("static-peer-discovery"), string("pass")]),
            record("check", vec![string("inventory-is-a-hint"), string("pass")]),
            record("check", vec![string("no-global-consistency-claim"), string("pass")]),
        ])]),
    ]);
    parse_inventory(&value, input.trust_root, input.key)
}

pub fn delegate_resource(
    resource: &Resource,
    capability: &str,
    signer: &str,
    trust_root: &str,
    key: &str,
) -> Result<Delegate> {
    validate_resource(resource)?;
    if capability.trim().is_empty() {
        return Err(MoltenError::invalid_harness("federation delegate capability must not be empty"));
    }
    let payload = record("federation-delegate-payload", vec![
        resource_value(resource),
        record("capability", vec![string(capability)]),
    ]);
    let signature = signature_record(&payload, signer, "federation-delegate", trust_root, key)?;
    let value = record("federation-delegate-v1", vec![payload, signature]);
    parse_delegate(&value, Some(&resource.resource_ref), Some(capability), trust_root, key)
}

pub fn parse_delegate(
    value: &IoValue,
    expected_resource_ref: Option<&str>,
    expected_capability: Option<&str>,
    trust_root: &str,
    key: &str,
) -> Result<Delegate> {
    let fields = value
        .collect_simple_record("federation-delegate-v1", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-delegate-v1 ...>"))?;
    let payload = value_to_iovalue(&fields[0]);
    let payload_fields = payload
        .collect_simple_record("federation-delegate-payload", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation delegate payload"))?;
    let resource = parse_resource(&value_to_iovalue(&payload_fields[0]))?;
    let capability = record_string(&payload_fields[1], "capability")?;
    if let Some(expected_resource_ref) = expected_resource_ref
        && resource.resource_ref != expected_resource_ref
    {
        return Err(MoltenError::invalid_harness("federation delegate resource binding mismatch"));
    }
    if let Some(expected_capability) = expected_capability
        && capability != expected_capability
    {
        return Err(MoltenError::invalid_harness("federation delegate capability mismatch"));
    }
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[1], &payload, "federation-delegate", trust_root, key)?;
    Ok(Delegate {
        delegate_ref: canonical_hash(value)?,
        resource_ref: resource.resource_ref,
        capability,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

fn parse_delegate_unverified(value: &IoValue) -> Result<Delegate> {
    let fields = value
        .collect_simple_record("federation-delegate-v1", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-delegate-v1 ...>"))?;
    let payload = value_to_iovalue(&fields[0]);
    let payload_fields = payload
        .collect_simple_record("federation-delegate-payload", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation delegate payload"))?;
    let resource = parse_resource(&value_to_iovalue(&payload_fields[0]))?;
    let capability = record_string(&payload_fields[1], "capability")?;
    let signature = value_to_iovalue(&fields[1]);
    let signature_fields = signature
        .collect_simple_record("signature", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected federation signature"))?;
    Ok(Delegate {
        delegate_ref: canonical_hash(value)?,
        resource_ref: resource.resource_ref,
        capability,
        signer: record_string(&signature_fields[0], "signer")?,
        trust_root: record_string(&signature_fields[2], "trust-root")?,
        value: value.clone(),
    })
}

pub fn parse_announcement(value: &IoValue, trust_root: &str, key: &str) -> Result<Announcement> {
    let fields = value
        .collect_simple_record("federation-announcement-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-announcement-v1 ...>"))?;
    require_schema(&fields[0], ANNOUNCEMENT_SCHEMA, "federation announcement schema")?;
    let payload = record_value(&fields[1], "payload")?;
    let (peer, resource, _policy_refs) = parse_announcement_payload(&payload)?;
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[2], &payload, ANNOUNCEMENT_PURPOSE, trust_root, key)?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "signed-announcement")?;
    require_check(&checks, "announcement-is-a-hint")?;
    Ok(Announcement {
        announcement_ref: canonical_hash(value)?,
        peer,
        resource,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

pub fn parse_inventory(value: &IoValue, trust_root: &str, key: &str) -> Result<Inventory> {
    let fields = value
        .collect_simple_record("federation-inventory-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <federation-inventory-v1 ...>"))?;
    require_schema(&fields[0], INVENTORY_SCHEMA, "federation inventory schema")?;
    let payload = record_value(&fields[1], "payload")?;
    let (peer, resources, delegates) = parse_inventory_payload(&payload)?;
    let (signer, actual_trust_root) =
        verify_signature_record(&fields[2], &payload, INVENTORY_PURPOSE, trust_root, key)?;
    let checks = parse_checks(&fields[3])?;
    require_check(&checks, "signed-inventory")?;
    require_check(&checks, "no-global-consistency-claim")?;
    Ok(Inventory {
        inventory_ref: canonical_hash(value)?,
        peer,
        resources,
        delegates,
        signer,
        trust_root: actual_trust_root,
        value: value.clone(),
    })
}

pub fn pull_ledger_inventory(input: &PullLedgerInventoryInput<'_>) -> Result<Pull> {
    pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
        source_root: input.source_root,
        dest_root: input.dest_root,
        inventory_value: input.inventory_value,
        trust_root: input.trust_root,
        key: input.key,
        policy: &PullPolicy::allowed_types(input.allowed_resource_types.to_vec()),
    })
}

pub fn pull_ledger_inventory_with_policy(input: &PullLedgerInventoryPolicyInput<'_>) -> Result<Pull> {
    let inventory = parse_inventory(input.inventory_value, input.trust_root, input.key)?;
    if inventory.resources.len() > input.policy.max_resources {
        return Ok(oversized_pull(&inventory));
    }
    let refs = PullEnv::new(input, &inventory)?.collect_refs()?;
    Ok(finish_pull(inventory, refs))
}

#[derive(Default)]
struct PullRefs {
    imported_refs: Vec<String>,
    skipped_refs: Vec<String>,
    denied_refs: Vec<String>,
}

impl PullRefs {
    fn deny(&mut self, resource: &Resource) -> Result<()> {
        push_bounded(&mut self.denied_refs, resource.resource_ref.clone(), MAX_RESOURCES, "federation denied refs")
    }

    fn skip(&mut self, resource: &Resource) -> Result<()> {
        push_bounded(&mut self.skipped_refs, resource.resource_ref.clone(), MAX_RESOURCES, "federation skipped refs")
    }

    fn import(&mut self, resource: &Resource) -> Result<()> {
        push_bounded(&mut self.imported_refs, resource.resource_ref.clone(), MAX_RESOURCES, "federation imported refs")
    }
}

struct PullEnv<'a, 'b> {
    input: &'a PullLedgerInventoryPolicyInput<'b>,
    inventory: &'a Inventory,
    existing: BtreeSet<String>,
    allowed: BtreeSet<String>,
}
