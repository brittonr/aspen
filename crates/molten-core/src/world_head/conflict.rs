use std::collections::BTreeSet;

use super::WorldHeadConflictMember;
use super::WorldHeadConflictSet;
use super::WorldHeadIssue;
use super::WorldHeadTransitionPlan;

const WORLD_HEAD_CONFLICT_IDENTITY_DOMAIN: &str = "molten.world-head-conflict.identity.v1";

pub fn classify_world_head_conflict(
    plans: &[WorldHeadTransitionPlan],
    maximum: u32,
) -> Result<Option<WorldHeadConflictSet>, Vec<WorldHeadIssue>> {
    if maximum == 0 {
        return Err(vec![WorldHeadIssue::InvalidBounds("max_conflicts")]);
    }
    let maximum = usize::try_from(maximum).map_err(|_| vec![WorldHeadIssue::InvalidBounds("max_conflicts")])?;
    if plans.len() < 2 {
        return Ok(None);
    }
    if plans.len() > maximum {
        return Err(vec![WorldHeadIssue::ConflictLimitExceeded]);
    }
    let Some(first_before) = plans[0].before.as_ref() else {
        return Err(vec![WorldHeadIssue::ConflictStateMismatch]);
    };
    if plans.iter().any(|plan| {
        plan.before.as_ref().is_none_or(|before| {
            before.branch_id != first_before.branch_id
                || before.head != first_before.head
                || before.generation != first_before.generation
                || before.policy_ref != first_before.policy_ref
        })
    }) {
        return Err(vec![WorldHeadIssue::ConflictStateMismatch]);
    }

    let mut unique = plans
        .iter()
        .map(|plan| WorldHeadConflictMember {
            claim_ref: plan.claim_ref.clone(),
            successor_head: plan.after.head.clone(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    unique.sort();
    let successor_count = unique.iter().map(|member| member.successor_head.clone()).collect::<BTreeSet<_>>().len();
    if successor_count < 2 {
        return Ok(None);
    }
    let conflict_ref = conflict_identity(first_before, &unique).map_err(|issue| vec![issue])?;
    Ok(Some(WorldHeadConflictSet {
        branch_id: first_before.branch_id.clone(),
        expected_head: first_before.head.clone(),
        expected_generation: first_before.generation,
        members: unique,
        conflict_ref,
    }))
}

fn conflict_identity(
    before: &super::WorldHeadState,
    members: &[WorldHeadConflictMember],
) -> Result<String, WorldHeadIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_HEAD_CONFLICT_IDENTITY_DOMAIN);
    update_bytes(&mut hasher, before.branch_id.as_str().as_bytes())?;
    update_bytes(&mut hasher, before.head.as_str().as_bytes())?;
    hasher.update(&before.generation.to_le_bytes());
    update_bytes(&mut hasher, before.policy_ref.as_str().as_bytes())?;
    update_length(&mut hasher, members.len())?;
    for member in members {
        update_bytes(&mut hasher, member.claim_ref.as_str().as_bytes())?;
        update_bytes(&mut hasher, member.successor_head.as_str().as_bytes())?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), WorldHeadIssue> {
    update_length(hasher, bytes.len())?;
    hasher.update(bytes);
    Ok(())
}

fn update_length(hasher: &mut blake3::Hasher, length: usize) -> Result<(), WorldHeadIssue> {
    let framed = u64::try_from(length).map_err(|_| WorldHeadIssue::InvalidBounds("conflict_identity_length"))?;
    hasher.update(&framed.to_le_bytes());
    Ok(())
}
