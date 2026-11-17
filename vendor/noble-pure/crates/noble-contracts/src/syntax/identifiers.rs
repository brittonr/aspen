pub(super) fn byte(
    bytes: &[u8],
    at: usize,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    let byte = match bytes.get(at) {
        Some(byte) => *byte,
        None => return Err(crate::internal(span)),
    };
    let is_letter = letter(byte);
    let is_suffix = at > 0 && suffix(byte);
    if !is_letter && !is_suffix {
        Err(crate::invalid(
            span,
            "identifier must begin with an ASCII letter or underscore",
        ))
    } else {
        Ok(())
    }
}

const fn letter(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

const fn suffix(byte: u8) -> bool {
    byte.is_ascii_digit() || byte == b'-' || byte == b'.'
}
