pub fn inventory_payload(
    inventory: &molten_core::federation::InventoryObservation,
) -> crate::error::Result<preserves::IOValue> {
    if inventory.entries.len() > molten_core::federation::MAX_FEDERATION_ENTRIES {
        return Err(invalid("inventory entry bound exceeded"));
    }
    Ok(crate::preserves_rail::record("federation-inventory-v1", vec![
        crate::preserves_rail::string(&inventory.peer_ref),
        crate::preserves_rail::string(&inventory.signer_ref),
        crate::preserves_rail::u64_value(inventory.generation),
        crate::preserves_rail::sequence(
            inventory
                .entries
                .iter()
                .map(|entry| {
                    crate::preserves_rail::record("federation-entry-v1", vec![
                        crate::preserves_rail::string(&entry.name),
                        crate::preserves_rail::string(&entry.head_ref),
                        crate::preserves_rail::string(&entry.ancestry_ref),
                        crate::preserves_rail::string(&entry.policy_ref),
                        crate::preserves_rail::u64_value(entry.bytes),
                        crate::preserves_rail::u64_value(entry.dag_nodes),
                        crate::preserves_rail::u64_value(entry.chunks),
                    ])
                })
                .collect(),
        ),
    ]))
}

// r[impl molten.federated_pull_runtime.receiver_pull]
pub fn verified_inventory(
    record: &super::network::VerifiedRecord,
    bounds: &molten_core::federation::PullBounds,
) -> crate::error::Result<molten_core::federation::InventoryObservation> {
    if record.record().purpose != molten_core::federation::RecordPurpose::Inventory {
        return Err(invalid("receiver inventory requires inventory signature domain"));
    }
    let fields = record
        .record()
        .payload
        .collect_simple_record("federation-inventory-v1", Some(4))
        .ok_or_else(|| invalid("malformed signed inventory"))?;
    let peer_ref = text(&fields[0])?;
    let signer_ref = text(&fields[1])?;
    if signer_ref != record.record().signature.metadata.signer_public_ref {
        return Err(invalid("inventory signer does not match verified key"));
    }
    let entries = fields[3].collect_sequence().ok_or_else(|| invalid("inventory entries must be sequence"))?;
    if entries.len() > bounds.inventory_entries || entries.len() > molten_core::federation::MAX_FEDERATION_ENTRIES {
        return Err(invalid("signed inventory exceeds receiver entry bound"));
    }
    let entries = entries
        .iter()
        .map(|entry| {
            let entry = crate::preserves_rail::value_to_iovalue(entry);
            let fields = entry
                .collect_simple_record("federation-entry-v1", Some(7))
                .ok_or_else(|| invalid("malformed inventory entry"))?;
            Ok(molten_core::federation::InventoryEntry {
                name: text(&fields[0])?,
                head_ref: text(&fields[1])?,
                ancestry_ref: text(&fields[2])?,
                policy_ref: text(&fields[3])?,
                bytes: integer(&fields[4])?,
                dag_nodes: integer(&fields[5])?,
                chunks: integer(&fields[6])?,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    Ok(molten_core::federation::InventoryObservation {
        peer_ref,
        signer_ref,
        generation: integer(&fields[2])?,
        issued_at: record.record().issued_at,
        expires_at: record.record().expires_at,
        encoded_bytes: record.transport().payload_bytes,
        entries,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegatePayload {
    pub subject_ref: String,
    pub delegate_ref: String,
    pub capability: String,
    pub policy_ref: String,
}

/// A verified remote claim remains a hint, never a local capability grant.
pub struct DelegateHint {
    pub payload: DelegatePayload,
    pub signer_ref: String,
    pub key_generation: u64,
    pub expires_at: u64,
    pub verification_ref: String,
}

pub fn delegate_payload(input: &DelegatePayload) -> crate::error::Result<preserves::IOValue> {
    validate_delegate(input)?;
    Ok(crate::preserves_rail::record("federation-delegate-v1", vec![
        crate::preserves_rail::string(&input.subject_ref),
        crate::preserves_rail::string(&input.delegate_ref),
        crate::preserves_rail::string(&input.capability),
        crate::preserves_rail::string(&input.policy_ref),
    ]))
}

pub fn verified_delegate(record: &super::network::VerifiedRecord) -> crate::error::Result<DelegateHint> {
    if record.record().purpose != molten_core::federation::RecordPurpose::Delegate {
        return Err(invalid("delegate requires its admitted signature domain"));
    }
    let fields = record
        .record()
        .payload
        .collect_simple_record("federation-delegate-v1", Some(4))
        .ok_or_else(|| invalid("malformed signed delegate record"))?;
    let payload = DelegatePayload {
        subject_ref: text(&fields[0])?,
        delegate_ref: text(&fields[1])?,
        capability: text(&fields[2])?,
        policy_ref: text(&fields[3])?,
    };
    validate_delegate(&payload)?;
    Ok(DelegateHint {
        payload,
        signer_ref: record.record().signature.metadata.signer_public_ref.clone(),
        key_generation: record.record().generation,
        expires_at: record.record().expires_at,
        verification_ref: record.verification().outcome_ref.clone(),
    })
}

fn validate_delegate(input: &DelegatePayload) -> crate::error::Result<()> {
    for reference in [&input.subject_ref, &input.delegate_ref, &input.policy_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    if input.capability.is_empty() || input.capability.len() > 128 || input.capability.chars().any(char::is_control) {
        return Err(invalid("malformed federation delegate capability"));
    }
    Ok(())
}

fn text(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<String> {
    let text = value.as_string().ok_or_else(|| invalid("expected inventory string"))?;
    if text.len() > 256 {
        return Err(invalid("inventory string exceeds bound"));
    }
    Ok(text.into_owned())
}
fn integer(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| invalid("expected inventory integer"))?
        .map_err(|_| invalid("inventory integer exceeds u64"))
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncementPayload {
    pub peer_ref: String,
    pub subject_ref: String,
    pub schema_ref: String,
    pub locator_ref: String,
    pub policy_ref: String,
}

pub fn announcement_payload(input: &AnnouncementPayload) -> crate::error::Result<preserves::IOValue> {
    validate_announcement(input)?;
    Ok(crate::preserves_rail::record("federation-announcement-v1", vec![
        crate::preserves_rail::string(&input.peer_ref),
        crate::preserves_rail::string(&input.subject_ref),
        crate::preserves_rail::string(&input.schema_ref),
        crate::preserves_rail::string(&input.locator_ref),
        crate::preserves_rail::string(&input.policy_ref),
    ]))
}

/// Parsed discovery metadata has no import, installation, pin or merge authority.
pub fn verified_announcement(record: &super::network::VerifiedRecord) -> crate::error::Result<AnnouncementPayload> {
    if record.record().purpose != molten_core::federation::RecordPurpose::Announcement {
        return Err(invalid("announcement requires its admitted signature domain"));
    }
    let fields = record
        .record()
        .payload
        .collect_simple_record("federation-announcement-v1", Some(5))
        .ok_or_else(|| invalid("malformed signed announcement"))?;
    let announcement = AnnouncementPayload {
        peer_ref: text(&fields[0])?,
        subject_ref: text(&fields[1])?,
        schema_ref: text(&fields[2])?,
        locator_ref: text(&fields[3])?,
        policy_ref: text(&fields[4])?,
    };
    validate_announcement(&announcement)?;
    Ok(announcement)
}
fn validate_announcement(input: &AnnouncementPayload) -> crate::error::Result<()> {
    for reference in [
        &input.peer_ref,
        &input.subject_ref,
        &input.schema_ref,
        &input.locator_ref,
        &input.policy_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}
