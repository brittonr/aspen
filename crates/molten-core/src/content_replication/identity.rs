use super::*;

const OPERATION_CONTEXT: &str = "onixresearch.molten.content-replication.operation.v1";
const ACTION_CONTEXT: &str = "onixresearch.molten.content-replication.action.v1";
const PLAN_CONTEXT: &str = "onixresearch.molten.content-replication.plan.v1";

pub struct OperationFrame<'a> {
    pub manifest: &'a Manifest,
    pub content: &'a ReplicaRule,
    pub source_peer: Option<&'a str>,
    pub target_peer: &'a str,
    pub kind: ActionKind,
    pub attempt: u32,
}

pub fn identify_operation(input: OperationFrame<'_>) -> Result<String, Issue> {
    let mut hasher = blake3::Hasher::new_derive_key(OPERATION_CONTEXT);
    update(&mut hasher, &input.manifest.service_id)?;
    update(&mut hasher, &input.manifest.generation.to_string())?;
    update(&mut hasher, &input.manifest.membership_epoch.to_string())?;
    update(&mut hasher, &input.manifest.placement_epoch.to_string())?;
    update(&mut hasher, &input.content.content_ref)?;
    update(&mut hasher, &input.content.manifest_ref)?;
    update(&mut hasher, &input.content.encoded_bytes.to_string())?;
    update(
        &mut hasher,
        if input.content.protected {
            "protected"
        } else {
            "ordinary"
        },
    )?;
    update(&mut hasher, input.content.transform_ref.as_deref().unwrap_or("identity"))?;
    update(&mut hasher, input.source_peer.unwrap_or("receiver-source-selection-pending"))?;
    update(&mut hasher, input.target_peer)?;
    update(&mut hasher, input.kind.as_str())?;
    update(&mut hasher, &input.attempt.to_string())?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_action(operation_id: &str, kind: ActionKind) -> Result<String, Issue> {
    let mut hasher = blake3::Hasher::new_derive_key(ACTION_CONTEXT);
    update(&mut hasher, operation_id)?;
    update(&mut hasher, kind.as_str())?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

pub fn identify_plan(manifest: &Manifest, observed_tick: u64, actions: &[Action]) -> Result<String, Issue> {
    let mut hasher = blake3::Hasher::new_derive_key(PLAN_CONTEXT);
    update(&mut hasher, &manifest.service_id)?;
    update(&mut hasher, &manifest.generation.to_string())?;
    update(&mut hasher, &manifest.membership_epoch.to_string())?;
    update(&mut hasher, &manifest.placement_epoch.to_string())?;
    update(&mut hasher, &observed_tick.to_string())?;
    for action in actions {
        update(&mut hasher, &action.action_id)?;
        update(&mut hasher, &action.operation_id)?;
        update(&mut hasher, action.kind.as_str())?;
        update(&mut hasher, &action.attempt.to_string())?;
        update(&mut hasher, &action.content_ref)?;
        update(&mut hasher, action.source_peer.as_deref().unwrap_or("none"))?;
        update(&mut hasher, &action.target_peer)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), Issue> {
    let length = u64::try_from(value.len()).map_err(|_| Issue::InvalidReference)?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}
