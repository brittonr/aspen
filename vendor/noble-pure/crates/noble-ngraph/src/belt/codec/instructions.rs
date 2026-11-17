pub(super) const MAX_BYTES: usize = 9;

pub(crate) fn next_instruction(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<crate::belt::types::Instruction, crate::belt::types::Refusal> {
    let opcode = take(bytes, cursor, 1)?[0];
    Ok(match opcode {
        0 => crate::belt::types::Instruction::Input(read_u32(bytes, cursor)?),
        1 => {
            let raw: [u8; 8] = take(bytes, cursor, 8)?
                .try_into()
                .map_err(|_| crate::belt::types::Refusal::Malformed)?;
            crate::belt::types::Instruction::Constant(u64::from_le_bytes(raw))
        }
        2 => crate::belt::types::Instruction::Add(
            take(bytes, cursor, 1)?[0],
            take(bytes, cursor, 1)?[0],
        ),
        3 => crate::belt::types::Instruction::Sub(
            take(bytes, cursor, 1)?[0],
            take(bytes, cursor, 1)?[0],
        ),
        4 => crate::belt::types::Instruction::Mul(
            take(bytes, cursor, 1)?[0],
            take(bytes, cursor, 1)?[0],
        ),
        5 => crate::belt::types::Instruction::Eq(
            take(bytes, cursor, 1)?[0],
            take(bytes, cursor, 1)?[0],
        ),
        6 => crate::belt::types::Instruction::Save(
            take(bytes, cursor, 1)?[0],
            take(bytes, cursor, 1)?[0],
        ),
        7 => crate::belt::types::Instruction::Load(take(bytes, cursor, 1)?[0]),
        8 => crate::belt::types::Instruction::Output(take(bytes, cursor, 1)?[0]),
        _ => return Err(crate::belt::types::Refusal::Malformed),
    })
}

pub(super) fn count(bytes: &[u8]) -> Result<usize, crate::belt::types::Refusal> {
    let mut cursor = 0usize;
    let mut count = 0usize;
    while cursor < bytes.len() {
        let _ = next_instruction(bytes, &mut cursor)?;
        count = count
            .checked_add(1)
            .ok_or(crate::belt::types::Refusal::InstructionBound)?;
        if count > crate::belt::MAX_INSTRUCTIONS {
            return Err(crate::belt::types::Refusal::InstructionBound);
        }
    }
    Ok(count)
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
