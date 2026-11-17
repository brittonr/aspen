pub(super) fn digest(
    ids: &dyn crate::identity::IdentityScheme,
    domain: &str,
    bytes: &[u8],
) -> Result<crate::ir::Digest, crate::belt::types::Refusal> {
    let mut hasher = ids.start();
    hasher.update(domain.as_bytes());
    hasher.update(&[0]);
    let len = u64::try_from(bytes.len()).map_err(|_| crate::belt::types::Refusal::ArtifactBound)?;
    hasher.update(&len.to_le_bytes());
    hasher.update(bytes);
    Ok(hasher.finish())
}

pub(super) struct ArtifactBinding {
    pub(super) graph_id: crate::ir::Digest,
    pub(super) belt_slots: u8,
    pub(super) save_slots: u8,
    pub(super) instruction_count: usize,
    pub(super) instruction_bytes_len: usize,
    pub(super) instruction_digest: crate::ir::Digest,
}

pub(super) fn artifact_binding(
    ids: &dyn crate::identity::IdentityScheme,
    binding: ArtifactBinding,
) -> Result<crate::ir::Digest, crate::belt::types::Refusal> {
    let ArtifactBinding {
        graph_id,
        belt_slots,
        save_slots,
        instruction_count,
        instruction_bytes_len,
        instruction_digest,
    } = binding;
    let mut hasher = ids.start();
    hasher.update(b"noble.ngraph.belt.artifact.v0\0");
    for revision in [
        super::super::FORMAT_REVISION,
        super::super::LOWERER_REVISION,
        super::super::CHECKER_REVISION,
    ] {
        let len =
            u64::try_from(revision.len()).map_err(|_| crate::belt::types::Refusal::Revision)?;
        hasher.update(&len.to_le_bytes());
        hasher.update(revision.as_bytes());
    }
    hasher.update(&graph_id);
    hasher.update(&[belt_slots, save_slots]);
    hasher.update(
        &u32::try_from(instruction_count)
            .map_err(|_| crate::belt::types::Refusal::InstructionBound)?
            .to_le_bytes(),
    );
    hasher.update(
        &u32::try_from(instruction_bytes_len)
            .map_err(|_| crate::belt::types::Refusal::ArtifactBound)?
            .to_le_bytes(),
    );
    hasher.update(&instruction_digest);
    Ok(hasher.finish())
}
