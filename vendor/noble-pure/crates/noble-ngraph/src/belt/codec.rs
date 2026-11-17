mod hashing;
pub(super) mod instructions;

const MAGIC: &[u8; 4] = b"NGB0";
const FIXED_HEADER: usize = 4
    + 1
    + super::FORMAT_REVISION.len()
    + 1
    + super::LOWERER_REVISION.len()
    + 1
    + super::CHECKER_REVISION.len()
    + 32
    + 1
    + 1
    + 4
    + 4
    + 32
    + 32;
/// Largest permitted encoded belt artifact, including its fixed header.
pub const MAX_ARTIFACT_BYTES: usize =
    FIXED_HEADER + super::MAX_INSTRUCTIONS * instructions::MAX_BYTES;

/// Bind an instruction body to its checked graph, revisions and machine bounds.
///
/// The body is structurally decoded here but not semantically accepted; use
/// [`super::check`] for the independent graph correspondence check.
pub fn encode(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    belt_slots: u8,
    save_slots: u8,
    instruction_bytes: &[u8],
) -> Result<alloc::vec::Vec<u8>, crate::belt::types::Refusal> {
    super::support::admit_graph(checked)?;
    validate_bounds(belt_slots, save_slots, instruction_bytes.len())?;
    let instruction_count = instructions::count(instruction_bytes)?;
    if instruction_count > super::MAX_INSTRUCTIONS {
        return Err(crate::belt::types::Refusal::InstructionBound);
    }
    let instruction_digest =
        hashing::digest(ids, "noble.ngraph.belt.instructions.v0", instruction_bytes)?;
    let graph_id = checked.id();
    let binding = hashing::artifact_binding(
        ids,
        hashing::ArtifactBinding {
            graph_id,
            belt_slots,
            save_slots,
            instruction_count,
            instruction_bytes_len: instruction_bytes.len(),
            instruction_digest,
        },
    )?;
    let total = FIXED_HEADER
        .checked_add(instruction_bytes.len())
        .filter(|length| *length <= MAX_ARTIFACT_BYTES)
        .ok_or(crate::belt::types::Refusal::ArtifactBound)?;
    let mut bytes = alloc::vec::Vec::with_capacity(total);
    bytes.extend_from_slice(MAGIC);
    for revision in [
        super::FORMAT_REVISION,
        super::LOWERER_REVISION,
        super::CHECKER_REVISION,
    ] {
        bytes.push(
            u8::try_from(revision.len()).map_err(|_| crate::belt::types::Refusal::Malformed)?,
        );
        bytes.extend_from_slice(revision.as_bytes());
    }
    bytes.extend_from_slice(&graph_id);
    bytes.extend_from_slice(&[belt_slots, save_slots]);
    bytes.extend_from_slice(
        &u32::try_from(instruction_count)
            .map_err(|_| crate::belt::types::Refusal::InstructionBound)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &u32::try_from(instruction_bytes.len())
            .map_err(|_| crate::belt::types::Refusal::ArtifactBound)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&instruction_digest);
    bytes.extend_from_slice(&binding);
    bytes.extend_from_slice(instruction_bytes);
    Ok(bytes)
}

/// Decode revisions, bounds and both content digests without accepting graph semantics.
pub fn decode<'a>(
    ids: &dyn crate::identity::IdentityScheme,
    bytes: &'a [u8],
) -> Result<crate::belt::types::ArtifactView<'a>, crate::belt::types::Refusal> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(crate::belt::types::Refusal::ArtifactBound);
    }
    let mut cursor = 0usize;
    if take(bytes, &mut cursor, MAGIC.len())? != MAGIC {
        return Err(crate::belt::types::Refusal::Revision);
    }
    read_revision(bytes, &mut cursor, super::FORMAT_REVISION)?;
    read_revision(bytes, &mut cursor, super::LOWERER_REVISION)?;
    read_revision(bytes, &mut cursor, super::CHECKER_REVISION)?;
    let graph_id = digest_field(bytes, &mut cursor)?;
    let belt_slots = take(bytes, &mut cursor, 1)?[0];
    let save_slots = take(bytes, &mut cursor, 1)?[0];
    let instruction_count = usize::try_from(read_u32(bytes, &mut cursor)?)
        .map_err(|_| crate::belt::types::Refusal::InstructionBound)?;
    let instruction_bytes_len = usize::try_from(read_u32(bytes, &mut cursor)?)
        .map_err(|_| crate::belt::types::Refusal::ArtifactBound)?;
    let declared_digest = digest_field(bytes, &mut cursor)?;
    let declared_binding = digest_field(bytes, &mut cursor)?;
    validate_bounds(belt_slots, save_slots, instruction_bytes_len)?;
    if instruction_count > super::MAX_INSTRUCTIONS {
        return Err(crate::belt::types::Refusal::InstructionBound);
    }
    if cursor != FIXED_HEADER {
        return Err(crate::belt::types::Refusal::Malformed);
    }
    let instruction_bytes = take(bytes, &mut cursor, instruction_bytes_len)?;
    if cursor != bytes.len() {
        return Err(crate::belt::types::Refusal::Malformed);
    }
    let observed_digest =
        hashing::digest(ids, "noble.ngraph.belt.instructions.v0", instruction_bytes)?;
    if observed_digest != declared_digest {
        return Err(crate::belt::types::Refusal::InstructionDigest);
    }
    let observed_count = instructions::count(instruction_bytes)?;
    if observed_count != instruction_count {
        return Err(crate::belt::types::Refusal::Malformed);
    }
    let observed_binding = hashing::artifact_binding(
        ids,
        hashing::ArtifactBinding {
            graph_id,
            belt_slots,
            save_slots,
            instruction_count,
            instruction_bytes_len,
            instruction_digest: declared_digest,
        },
    )?;
    if observed_binding != declared_binding {
        return Err(crate::belt::types::Refusal::ArtifactBinding);
    }
    Ok(crate::belt::types::ArtifactView {
        bytes,
        graph_id,
        belt_slots,
        save_slots,
        instruction_bytes,
        instruction_digest: observed_digest,
        artifact_binding: observed_binding,
        instruction_count,
    })
}

fn validate_bounds(
    belt_slots: u8,
    save_slots: u8,
    instruction_bytes_len: usize,
) -> Result<(), crate::belt::types::Refusal> {
    if !(super::MIN_CAPACITY..=super::MAX_CAPACITY).contains(&belt_slots) {
        return Err(crate::belt::types::Refusal::Capacity);
    }
    if save_slots > super::MAX_SAVE_SLOTS {
        return Err(crate::belt::types::Refusal::SaveSlots);
    }
    let max_instruction_bytes = super::MAX_INSTRUCTIONS
        .checked_mul(instructions::MAX_BYTES)
        .ok_or(crate::belt::types::Refusal::ArtifactBound)?;
    if instruction_bytes_len > max_instruction_bytes {
        return Err(crate::belt::types::Refusal::ArtifactBound);
    }
    Ok(())
}

fn read_revision(
    bytes: &[u8],
    cursor: &mut usize,
    expected: &str,
) -> Result<(), crate::belt::types::Refusal> {
    let len = usize::from(take(bytes, cursor, 1)?[0]);
    if take(bytes, cursor, len)? != expected.as_bytes() {
        return Err(crate::belt::types::Refusal::Revision);
    }
    Ok(())
}

fn digest_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<crate::ir::Digest, crate::belt::types::Refusal> {
    take(bytes, cursor, 32)?
        .try_into()
        .map_err(|_| crate::belt::types::Refusal::Malformed)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, crate::belt::types::Refusal> {
    let raw: [u8; 4] = take(bytes, cursor, 4)?
        .try_into()
        .map_err(|_| crate::belt::types::Refusal::Malformed)?;
    Ok(u32::from_le_bytes(raw))
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], crate::belt::types::Refusal> {
    let end = cursor
        .checked_add(len)
        .ok_or(crate::belt::types::Refusal::Malformed)?;
    let field = bytes
        .get(*cursor..end)
        .ok_or(crate::belt::types::Refusal::Malformed)?;
    *cursor = end;
    Ok(field)
}
