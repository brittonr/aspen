const fn initial(byte: u8) -> bool {
    byte.is_ascii_alphabetic()
        || matches!(
            byte,
            b'_' | b'+' | b'-' | b'*' | b'/' | b'=' | b'<' | b'>' | b'?' | b'!'
        )
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; decimal and ASCII grammar scans charge each byte and reject malformed tokens with diagnostics, while checked integer parsing preserves the signed-I64 boundary."
)]
pub(in crate::source) fn classify(
    bytes: &[u8],
    span: crate::Span,
    meter: &mut crate::Meter,
    declared: bool,
) -> Result<super::TokenKind, crate::Diagnostic> {
    if bytes == b"def" {
        return Ok(super::TokenKind::Def);
    }
    if bytes == b"true" {
        return Ok(super::TokenKind::Literal(
            noble_kernel::untrusted::Lit::Bool(true),
        ));
    }
    if bytes == b"false" {
        return Ok(super::TokenKind::Literal(
            noble_kernel::untrusted::Lit::Bool(false),
        ));
    }
    let digits = if bytes.first() == Some(&b'-') {
        &bytes[1..]
    } else {
        bytes
    };
    let mut is_decimal = !digits.is_empty();
    let mut at = 0usize;
    let mut failure = None;
    while let Some(byte) = digits.get(at).copied() {
        if let Err(problem) = meter.charge(1, span) {
            failure = Some(problem);
            break;
        }
        is_decimal &= byte.is_ascii_digit();
        at += 1;
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    if is_decimal && (digits.len() == 1 || digits.first() != Some(&b'0')) {
        return match attempt!(crate::syntax::integer(bytes, span, meter)) {
            Some(value) => Ok(super::TokenKind::Literal(
                noble_kernel::untrusted::Lit::I64(value),
            )),
            None => Err(crate::internal(span)),
        };
    }
    if !bytes.first().copied().map(initial).unwrap_or(false) {
        return Err(crate::invalid(
            span,
            "token is neither a complete I64 literal nor an ASCII word",
        ));
    }
    attempt!(word(bytes, span, meter, declared));
    Ok(super::TokenKind::Word(bytes.to_vec()))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every candidate word byte is work-charged before its ASCII grammar check, with the first rejection returned after the finite slice scan."
)]
fn word(
    bytes: &[u8],
    span: crate::Span,
    meter: &mut crate::Meter,
    declared: bool,
) -> Result<(), crate::Diagnostic> {
    let mut at = 0usize;
    let mut failure = None;
    while let Some(byte) = bytes.get(at).copied() {
        if let Err(problem) = meter.charge(1, span) {
            failure = Some(problem);
            break;
        }
        if !(initial(byte)
            || byte.is_ascii_digit()
            || byte == b'.'
            || (declared && matches!(byte, b'@' | b',' | b':')))
        {
            failure = Some(crate::invalid(
                span,
                "word contains a character outside the ASCII token grammar",
            ));
            break;
        }
        at += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}
