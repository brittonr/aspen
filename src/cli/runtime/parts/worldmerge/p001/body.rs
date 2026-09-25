
fn conservative_profile(profile_ref: &str, policy_ref: &str) -> Result<WorldMergeProfile> {
    Ok(WorldMergeProfile {
        profile_ref: WorldMergeProfileRef::new(profile_ref.to_string())
            .map_err(|error| MoltenError::invalid_harness(format!("invalid merge profile ref: {error}")))?,
        policy_ref: WorldMergePolicyRef::new(policy_ref.to_string())
            .map_err(|error| MoltenError::invalid_harness(format!("invalid merge policy ref: {error}")))?,
        root_modes: BTreeMap::from([
            (RootKind::Artifact, WorldMergeMode::IdenticalOnly),
            (RootKind::Schema, WorldMergeMode::IdenticalOnly),
            (RootKind::DurableState, WorldMergeMode::AncestorReplacement),
            (RootKind::RuntimeProfile, WorldMergeMode::IdenticalOnly),
            (RootKind::Policy, WorldMergeMode::IdenticalOnly),
        ]),
        migrations: BTreeMap::new(),
        handlers: BTreeMap::new(),
    })
}

fn operator_world_bounds() -> molten_core::world_commit::WorldCommitBounds {
    molten_core::world_commit::WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

fn parse_commit_ref(value: &str) -> Result<WorldCommitRef> {
    WorldCommitRef::new(value.to_string())
        .map_err(|error| MoltenError::invalid_harness(format!("invalid world commit ref: {error:?}")))
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
