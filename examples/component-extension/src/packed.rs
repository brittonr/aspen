//! Bounded packed byte sequences shared by the admitted callback projection and encoder.

const ENCODINGS: [(u32, u8); 256] = encodings();

const fn encodings() -> [(u32, u8); 256] {
    let mut table = [(0, 0); 256];
    let mut byte = 0u32;
    let mut index = 0;
    while index < 256 {
        table[index] = if byte == 0 {
            (0x00b0, 2)
        } else if byte < 128 {
            (0x01b0 | (byte << 16), 3)
        } else {
            (0x02b0 | (byte << 24), 4)
        };
        byte += 1;
        index += 1;
    }
    table
}

pub(super) fn write(output: &mut alloc::vec::Vec<u8>, bytes: &[u8]) {
    let maximum = bytes.len().checked_mul(4).and_then(|size| size.checked_add(3)).expect("packed value capacity");
    output.reserve(maximum);
    output.push(0xb5);
    let start = output.len();
    let mut count = 0;
    let target = unsafe { output.as_mut_ptr().add(start) };
    let mut remaining = bytes;
    while let Some((&byte, tail)) = remaining.split_first() {
        // A successful group consumes four bytes and writes twelve; the
        // reservation covers at least sixteen bytes for those four inputs.
        if unsafe { write_ascii_group(remaining, target.add(count)) }.is_some() {
            count += 12;
            remaining = unsafe { remaining.get_unchecked(4..) };
            continue;
        }
        let (word, size) = ENCODINGS[usize::from(byte)];
        // At most four initialized bytes per input byte; reservation includes
        // opener, closers and the spare byte after a three-byte integer.
        unsafe {
            target.add(count).cast::<u32>().write_unaligned(word.to_le());
        }
        count += usize::from(size);
        remaining = tail;
    }
    unsafe {
        output.set_len(start + count);
    }
    output.push(0x84);
    output.push(0x84);
}

// SAFETY: target has room for twelve bytes whenever bytes contains four inputs.
// A non-ASCII or short group returns None without writing anything.
#[inline(always)]
unsafe fn write_ascii_group(bytes: &[u8], target: *mut u8) -> Option<()> {
    if bytes.len() < 4 {
        return None;
    }
    let word = u32::from_le(unsafe { bytes.as_ptr().cast::<u32>().read_unaligned() });
    let zero_bytes = word.wrapping_sub(0x01010101) & !word & 0x80808080;
    if word & 0x80808080 != 0 || zero_bytes != 0 {
        return None;
    }
    let first = 0xb00001b0 | ((word & 0xff) << 16);
    let second = 0x01b00001 | (word & 0xff00);
    let third = 0x0001b000 | ((word >> 16) & 0xff) | (word & 0xff000000);
    unsafe {
        target.cast::<u32>().write_unaligned(first.to_le());
        target.add(4).cast::<u32>().write_unaligned(second.to_le());
        target.add(8).cast::<u32>().write_unaligned(third.to_le());
    }
    Some(())
}

pub(super) fn decode(bytes: &[u8], position: &mut usize) -> Result<alloc::vec::Vec<u8>, alloc::string::String> {
    if bytes.get(*position) != Some(&0xb5) {
        return Err(super::codec::fail());
    }
    let mut remaining = bytes.get(*position + 1..).ok_or_else(super::codec::fail)?;
    let mut result = alloc::vec::Vec::<u8>::with_capacity(remaining.len() / 2);
    let mut count = 0;
    loop {
        if remaining.first() == Some(&0x84) {
            *position = bytes.len() - remaining.len() + 1;
            break;
        }
        if let Some(word) = ascii_run(remaining) {
            // Twelve admitted input bytes produce four initialized output bytes;
            // the complete remaining input reserved at least six output slots.
            unsafe {
                result.as_mut_ptr().add(count).cast::<u32>().write_unaligned(word.to_le());
            }
            count += 4;
            remaining = unsafe { remaining.get_unchecked(12..) };
            continue;
        }
        let (byte, consumed) = integer(remaining)?;
        // Each accepted integer consumes at least two input bytes, bounding
        // initialized output slots by the reservation from remaining input.
        unsafe {
            result.as_mut_ptr().add(count).write(byte);
        }
        count += 1;
        remaining = unsafe { remaining.get_unchecked(consumed..) };
    }
    unsafe {
        result.set_len(count);
    }
    Ok(result)
}

#[inline(always)]
fn ascii_run(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 12 {
        return None;
    }
    // Exactly twelve initialized bytes hold four three-byte canonical integers.
    let (first, last) = unsafe {
        (
            u64::from_le(bytes.as_ptr().cast::<u64>().read_unaligned()),
            u32::from_le(bytes.as_ptr().add(8).cast::<u32>().read_unaligned()),
        )
    };
    if first & 0xffff80ffff80ffff != 0x01b00001b00001b0 || last & 0x80ffff80 != 0x0001b000 {
        return None;
    }
    let first = first.to_le_bytes();
    let last = last.to_le_bytes();
    let word = u32::from_le_bytes([first[2], first[5], last[0], last[3]]);
    let zero_bytes = word.wrapping_sub(0x01010101) & !word & 0x80808080;
    if zero_bytes != 0 {
        return None;
    }
    Some(word)
}

#[inline(always)]
fn integer(bytes: &[u8]) -> Result<(u8, usize), alloc::string::String> {
    if bytes.len() >= 4 {
        let word = u32::from_le(unsafe { bytes.as_ptr().cast::<u32>().read_unaligned() });
        let byte = word.to_le_bytes()[2];
        return match word & 0xffff {
            0x00b0 => Ok((0, 2)),
            0x01b0 if byte.wrapping_sub(1) < 127 => Ok((byte, 3)),
            0x02b0 if byte == 0 && word >> 24 >= 128 => Ok((word.to_le_bytes()[3], 4)),
            _ => Err(super::codec::fail()),
        };
    }
    match bytes {
        [0xb0, 0, ..] => Ok((0, 2)),
        [0xb0, 1, byte @ 1..=127, ..] => Ok((*byte, 3)),
        _ => Err(super::codec::fail()),
    }
}
