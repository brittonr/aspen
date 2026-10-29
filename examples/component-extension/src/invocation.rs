//! Projection of a callback envelope already admitted by the product executor.
//! Only the selected value is decoded; metadata and unrelated values stay borrowed.

pub(super) struct Projection<'a> {
    pub callback: &'a [u8],
    pub value: Option<alloc::vec::Vec<u8>>,
}

pub(super) fn decode(bytes: &[u8]) -> Result<Projection<'_>, alloc::string::String> {
    if bytes.len() > super::codec::MAX_BYTES
        || bytes.first() != Some(&0xb4)
        || !bytes.ends_with(b"\xb1\x27preserves-packed-materialized-values-v2\x84")
    {
        return Err(super::codec::fail());
    }
    let mut position = 1;
    if super::codec::read_atom(bytes, &mut position, 0xb3)? != b"native-callback-envelope-v3"
        || super::codec::read_atom(bytes, &mut position, 0xb1)?
            != b"molten.system-extension.native-callback-envelope.v3"
    {
        return Err(super::codec::fail());
    }
    for _ in 1..8 {
        super::codec::skip(bytes, &mut position, 0)?;
    }
    let callback = super::codec::read_atom(bytes, &mut position, 0xb1)?;
    let selected = match callback {
        b"checkpoint" => Some(14),
        b"request" | b"message" | b"recover" => Some(11),
        _ => None,
    };
    let value = if let Some(selected) = selected {
        for _ in 9..selected {
            super::codec::skip(bytes, &mut position, 0)?;
        }
        super::codec::admitted_value(bytes, &mut position)?
    } else {
        None
    };
    Ok(Projection { callback, value })
}
