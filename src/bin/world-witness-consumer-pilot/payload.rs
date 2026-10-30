const SIGNATURE_THRESHOLD: usize = 1;
const ALLOW_RECOVERY: bool = false;
const REQUIRE_INDEPENDENT_RECOVERY: bool = true;

/// Exact head-policy projection; only the policy's self-reference is excluded.
pub fn policy() -> super::Result<(molten_core::world_head::WorldHeadPolicy, Vec<u8>)> {
    let classes = std::collections::BTreeSet::from([molten_core::world_head::WorldBranchClass::Local]);
    let purposes = std::collections::BTreeSet::from([
        molten_core::world_head::WorldHeadPurpose::Create,
        molten_core::world_head::WorldHeadPurpose::Advance,
    ]);
    let roles = std::collections::BTreeSet::from([molten_core::world_head::WorldHeadSignerRole::Maintainer]);
    let bytes = packed(&molten::preserves_rail::record("molten-private-witness-head-policy-v1", vec![
        molten::preserves_rail::sequence(
            classes.iter().map(|value| molten::preserves_rail::string(value.as_str())).collect(),
        ),
        molten::preserves_rail::sequence(
            purposes.iter().map(|value| molten::preserves_rail::string(value.as_str())).collect(),
        ),
        molten::preserves_rail::sequence(
            roles.iter().map(|value| molten::preserves_rail::string(value.as_str())).collect(),
        ),
        molten::preserves_rail::u64_value(SIGNATURE_THRESHOLD as u64),
        molten::preserves_rail::u64_value(molten_core::world_head::MAX_WORLD_HEAD_CONFLICTS as u64),
        molten::preserves_rail::bool_value(ALLOW_RECOVERY),
        molten::preserves_rail::bool_value(REQUIRE_INDEPENDENT_RECOVERY),
    ]))?;
    let policy_ref =
        molten_core::world_head::WorldHeadPolicyRef::new(molten::preserves_rail::content_ref_from_bytes(&bytes))
            .map_err(|error| format!("canonical policy digest: {error:?}"))?;
    Ok((
        molten_core::world_head::WorldHeadPolicy {
            policy_ref,
            allowed_branch_classes: classes,
            allowed_purposes: purposes,
            allowed_signer_roles: roles,
            signature_threshold: SIGNATURE_THRESHOLD,
            max_conflicts: molten_core::world_head::MAX_WORLD_HEAD_CONFLICTS,
            allow_recovery: ALLOW_RECOVERY,
            require_independent_recovery_currentness: REQUIRE_INDEPENDENT_RECOVERY,
        },
        bytes,
    ))
}

/// An opaque content-addressed fixture, deliberately not a captured WorldCommit.
pub fn commit(generation: u64) -> super::Result<(molten_core::world_commit::WorldCommitRef, Vec<u8>)> {
    let mut payload = commit_step(1, None)?;
    for step in 2..=generation {
        payload = commit_step(step, Some(&payload.0))?;
    }
    Ok(payload)
}

fn commit_step(
    generation: u64,
    parent: Option<&molten_core::world_commit::WorldCommitRef>,
) -> super::Result<(molten_core::world_commit::WorldCommitRef, Vec<u8>)> {
    let parents = parent.map_or_else(Vec::new, |parent| vec![molten::preserves_rail::string(parent.as_str())]);
    let bytes = packed(&molten::preserves_rail::record("molten-private-witness-opaque-commit-v1", vec![
        molten::preserves_rail::string("opaque-artifact-only-not-world-capture-or-admission"),
        molten::preserves_rail::u64_value(generation),
        molten::preserves_rail::sequence(parents),
        molten::preserves_rail::record("fixture-state", vec![molten::preserves_rail::u64_value(generation)]),
    ]))?;
    let reference =
        molten_core::world_commit::WorldCommitRef::new(molten::preserves_rail::content_ref_from_bytes(&bytes))
            .map_err(|error| format!("canonical opaque fixture digest: {error:?}"))?;
    Ok((reference, bytes))
}

pub fn crypto_config(kind: &str) -> super::Result<(String, Vec<u8>)> {
    let bytes = packed(&molten::preserves_rail::record("molten-private-witness-crypto-config-v1", vec![
        molten::preserves_rail::string(kind),
        molten::preserves_rail::string("ed25519"),
        molten::preserves_rail::string("private-same-host-nonproduction-fixture"),
    ]))?;
    Ok((molten::preserves_rail::content_ref_from_bytes(&bytes), bytes))
}

pub fn materialize(root: &molten::node_state::DirectoryRoot) -> super::Result<()> {
    root.write_new(&molten::node_state::RelativePath::parse("pilot-head-policy.preserves")?, &policy()?.1)?;
    for generation in 1..=2 {
        let path = molten::node_state::RelativePath::parse(&format!("pilot-opaque-commit-{generation}.preserves"))?;
        root.write_new(&path, &commit(generation)?.1)?;
    }
    for kind in [
        "private-pilot-ed25519",
        "private-pilot-entropy",
        "private-pilot-backend",
    ] {
        let path = molten::node_state::RelativePath::parse(&format!("{kind}.preserves"))?;
        root.write_new(&path, &crypto_config(kind)?.1)?;
    }
    Ok(())
}

fn packed(value: &preserves::IOValue) -> super::Result<Vec<u8>> {
    Ok(molten::preserves_rail::canonical_bytes(value)?)
}
