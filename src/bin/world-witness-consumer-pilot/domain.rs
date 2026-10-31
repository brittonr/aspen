pub fn crypto_config_ref(kind: &str) -> super::Result<String> {
    Ok(super::payload::crypto_config(kind)?.0)
}

pub fn branch() -> super::Result<molten_core::world_head::WorldBranchId> {
    molten_core::world_head::WorldBranchId::new("main").map_err(|error| format!("fixed pilot branch: {error:?}").into())
}

pub fn policy_ref() -> super::Result<molten_core::world_head::WorldHeadPolicyRef> {
    Ok(policy()?.policy_ref)
}

pub fn commit(generation: u64) -> super::Result<molten_core::world_commit::WorldCommitRef> {
    Ok(super::payload::commit(generation)?.0)
}

pub fn claim(generation: u64) -> super::Result<molten_core::world_head::WorldHeadClaim> {
    successor(generation, commit(generation)?)
}

/// A competing successor reserved externally by a writer whose local CAS never ran.
pub fn abandoned_claim(generation: u64) -> super::Result<molten_core::world_head::WorldHeadClaim> {
    successor(generation, super::payload::abandoned(generation)?.0)
}

fn successor(
    generation: u64,
    successor_head: molten_core::world_commit::WorldCommitRef,
) -> super::Result<molten_core::world_head::WorldHeadClaim> {
    if !(1..=super::payload::GENERATIONS).contains(&generation) {
        return Err("pilot supports exactly generations 1 through 3".into());
    }
    Ok(molten_core::world_head::WorldHeadClaim {
        branch_id: branch()?,
        branch_class: molten_core::world_head::WorldBranchClass::Local,
        expected_head: if generation > 1 {
            Some(commit(generation - 1)?)
        } else {
            None
        },
        successor_head,
        expected_generation: generation - 1,
        successor_generation: generation,
        purpose: if generation == 1 {
            molten_core::world_head::WorldHeadPurpose::Create
        } else {
            molten_core::world_head::WorldHeadPurpose::Advance
        },
        policy_ref: policy_ref()?,
        source_heads: Vec::new(),
    })
}

pub fn policy() -> super::Result<molten_core::world_head::WorldHeadPolicy> {
    Ok(super::payload::policy()?.0)
}

pub fn history() -> super::Result<Vec<molten_core::world_head::WorldCommitHistoryNode>> {
    (1..=super::payload::GENERATIONS)
        .map(|generation| {
            let parents = if generation > 1 {
                vec![commit(generation - 1)?]
            } else {
                Vec::new()
            };
            Ok(molten_core::world_head::WorldCommitHistoryNode {
                commit: commit(generation)?,
                parents,
            })
        })
        .collect()
}

/// Shell wall-clock read through the admitted fabric-time live clock.
pub fn now() -> super::Result<u64> {
    Ok(molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)?)
}
