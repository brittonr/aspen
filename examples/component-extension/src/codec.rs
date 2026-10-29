//! Closed packed-Preserves callback codec, not a general Preserves runtime.
//! Input is independently canonicalized/admitted by the product executor.

pub(super) const MAX_BYTES: usize = 65_536;

pub(super) fn fail() -> alloc::string::String {
    alloc::string::String::from("invalid canonical callback envelope")
}

#[inline(always)]
fn length(bytes: &[u8], position: &mut usize) -> Result<usize, alloc::string::String> {
    let first = *bytes.get(*position).ok_or_else(fail)?;
    *position += 1;
    if first < 128 {
        return Ok(usize::from(first));
    }
    let mut value = usize::from(first & 127);
    for shift in [7, 14, 21] {
        let byte = *bytes.get(*position).ok_or_else(fail)?;
        *position += 1;
        value |= usize::from(byte & 127) << shift;
        if byte < 128 {
            if byte == 0 {
                return Err(fail());
            }
            return Ok(value);
        }
    }
    Err(fail())
}

pub(super) fn skip(bytes: &[u8], position: &mut usize, depth: usize) -> Result<(), alloc::string::String> {
    let mut nesting = depth;
    loop {
        let tag = *bytes.get(*position).ok_or_else(fail)?;
        if nesting > 32 && tag != 0x84 {
            return Err(fail());
        }
        *position += 1;
        match tag {
            0x80 | 0x81 => {}
            0x87 | 0xb0..=0xb3 => {
                let size = length(bytes, position)?;
                *position = position.checked_add(size).ok_or_else(fail)?;
                if *position > bytes.len() {
                    return Err(fail());
                }
            }
            0xb4..=0xb7 => {
                if nesting > 32 {
                    return Err(fail());
                }
                nesting += 1;
            }
            0x84 if nesting > depth => nesting -= 1,
            _ => return Err(fail()),
        }
        if nesting == depth {
            return Ok(());
        }
    }
}

pub fn atom(bytes: &[u8], tag: u8) -> Result<&[u8], alloc::string::String> {
    let mut position = 0;
    let value = read_atom(bytes, &mut position, tag)?;
    if position != bytes.len() {
        return Err(fail());
    }
    Ok(value)
}

pub(super) fn read_atom<'a>(bytes: &'a [u8], position: &mut usize, tag: u8) -> Result<&'a [u8], alloc::string::String> {
    if bytes.get(*position) != Some(&tag) {
        return Err(fail());
    }
    *position += 1;
    let size = length(bytes, position)?;
    let end = position.checked_add(size).ok_or_else(fail)?;
    let value = bytes.get(*position..end).ok_or_else(fail)?;
    *position = end;
    Ok(value)
}

pub fn record<'a, const N: usize>(bytes: &'a [u8], label: &str) -> Result<[&'a [u8]; N], alloc::string::String> {
    if bytes.len() > MAX_BYTES || bytes.first() != Some(&0xb4) {
        return Err(fail());
    }
    let mut position = 1;
    let start = position;
    skip(bytes, &mut position, 0)?;
    if atom(&bytes[start..position], 0xb3)? != label.as_bytes() {
        return Err(fail());
    }
    let mut fields = [&[][..]; N];
    for field in &mut fields {
        let start = position;
        skip(bytes, &mut position, 0)?;
        *field = &bytes[start..position];
    }
    if bytes.get(position) != Some(&0x84) || position + 1 != bytes.len() {
        return Err(fail());
    }
    Ok(fields)
}

/// Decode materialized input admitted by the product before guest invocation.
/// `Executor::materialize_inputs` checks bounds, BLAKE3 identity and requested ref.
pub(super) fn admitted_value(
    bytes: &[u8],
    position: &mut usize,
) -> Result<Option<alloc::vec::Vec<u8>>, alloc::string::String> {
    if bytes.len() > MAX_BYTES {
        return Err(fail());
    }
    let remaining = bytes.get(*position..).ok_or_else(fail)?;
    if remaining.starts_with(b"\xb4\xb3\x04none\x84") {
        *position += 8;
        return Ok(None);
    }
    const PREFIX: &[u8] = b"\xb4\xb3\x04some\xb4\xb3\x18native-callback-value-v2\xb1\x47";
    let value = remaining.strip_prefix(PREFIX).ok_or_else(fail)?;
    value.get(..71).ok_or_else(fail)?;
    *position += PREFIX.len() + 71;
    let content = super::packed::decode(bytes, position)?;
    if bytes.get(*position..*position + 2) != Some(&[0x84, 0x84]) {
        return Err(fail());
    }
    *position += 2;
    Ok(Some(content))
}

pub fn write_atom(output: &mut alloc::vec::Vec<u8>, tag: u8, bytes: &[u8]) {
    output.push(tag);
    let mut size = bytes.len();
    loop {
        if size < 128 {
            output.push(size as u8);
            break;
        }
        output.push((size & 127) as u8 | 128);
        size >>= 7;
    }
    output.extend_from_slice(bytes);
}

pub fn start_record(output: &mut alloc::vec::Vec<u8>, label: &str) {
    output.push(0xb4);
    write_atom(output, 0xb3, label.as_bytes());
}

pub fn write_value(output: &mut alloc::vec::Vec<u8>, bytes: &[u8]) {
    start_record(output, "native-callback-value-v2");
    write_atom(output, 0xb1, &super::identity::reference(bytes));
    super::packed::write(output, bytes);
}

fn write_optional(output: &mut alloc::vec::Vec<u8>, bytes: Option<&[u8]>) {
    start_record(output, if bytes.is_some() { "some" } else { "none" });
    if let Some(bytes) = bytes {
        write_value(output, bytes);
    }
    output.push(0x84);
}

pub fn outcome(output: Option<&[u8]>, state: Option<&[u8]>, checkpoint: Option<&[u8]>) -> alloc::vec::Vec<u8> {
    let content_bytes =
        output.map_or(0, <[u8]>::len) + state.map_or(0, <[u8]>::len) + checkpoint.map_or(0, <[u8]>::len);
    let value_count = usize::from(output.is_some()) + usize::from(state.is_some()) + usize::from(checkpoint.is_some());
    let mut bytes = alloc::vec::Vec::with_capacity(128 + value_count * 112 + content_bytes * 4);
    start_record(&mut bytes, "native-callback-outcome-v3");
    write_atom(&mut bytes, 0xb1, b"molten.system-extension.native-callback-outcome.v3");
    bytes.push(0xb5);
    let output_start = bytes.len();
    if let Some(output) = output {
        write_value(&mut bytes, output);
    }
    let output_end = bytes.len();
    bytes.push(0x84);
    // Effects performed by the effect-request host are already canonically
    // admitted and recorded there. Do not duplicate them as routed typed effects.
    bytes.extend_from_slice(&[0xb5, 0x84]);
    let is_state_reused = match (output, state) {
        (Some(output), Some(state)) => core::ptr::eq(output, state) || output == state,
        _ => false,
    };
    if is_state_reused {
        start_record(&mut bytes, "some");
        bytes.extend_from_within(output_start..output_end);
        bytes.push(0x84);
    } else {
        write_optional(&mut bytes, state);
    }
    write_optional(&mut bytes, checkpoint);
    write_atom(&mut bytes, 0xb1, b"healthy");
    bytes.push(0x84);
    bytes
}
