use super::super::WorldMergeConflict;
use super::super::WorldMergeIssue;
use super::super::WorldMergePlanRef;
use super::super::WorldMergeRequest;
use super::super::WorldMergedRoot;

const WORLD_MERGE_PLAN_IDENTITY_DOMAIN: &str = "molten.world-merge-plan.identity.v1";

pub(in crate::world_merge) fn identify_plan(
    request: &WorldMergeRequest,
    outputs: &[WorldMergedRoot],
    conflicts: &[WorldMergeConflict],
) -> Result<WorldMergePlanRef, WorldMergeIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_MERGE_PLAN_IDENTITY_DOMAIN);
    update_bytes(&mut hasher, request.base_head.as_str().as_bytes())?;
    update_bytes(&mut hasher, request.profile.profile_ref.as_str().as_bytes())?;
    for source in &request.source_heads {
        update_bytes(&mut hasher, source.as_str().as_bytes())?;
    }
    for output in outputs {
        update_bytes(&mut hasher, output.kind.as_str().as_bytes())?;
        if let Some(root) = &output.selected_root {
            update_bytes(&mut hasher, root.as_str().as_bytes())?;
        }
        for (key, value) in &output.generated_values {
            update_bytes(&mut hasher, key.as_bytes())?;
            update_bytes(&mut hasher, value)?;
        }
        if let Some(bytes) = &output.generated_bytes {
            update_bytes(&mut hasher, bytes)?;
        }
    }
    for conflict in conflicts {
        update_bytes(&mut hasher, conflict.kind.as_str().as_bytes())?;
        update_bytes(&mut hasher, conflict.key.as_deref().unwrap_or("").as_bytes())?;
        update_bytes(&mut hasher, conflict.code.as_bytes())?;
    }
    WorldMergePlanRef::new(format!("blake3:{}", hasher.finalize().to_hex()))
        .map_err(|_| WorldMergeIssue::InvalidBounds("plan_identity"))
}

fn update_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), WorldMergeIssue> {
    let length = u64::try_from(bytes.len()).map_err(|_| WorldMergeIssue::InvalidBounds("identity_length"))?;
    hasher.update(&length.to_le_bytes());
    hasher.update(bytes);
    Ok(())
}
