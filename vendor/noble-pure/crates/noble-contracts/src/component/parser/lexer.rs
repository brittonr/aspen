const MAX_BYTES: usize = 65_536;
const MAX_TOKENS: usize = 8192;
const MAX_COMMENT_DEPTH: u32 = 64;

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; UTF-8, admitted byte/node counts, scanner progress and token boundaries are fallible ingress checks; the finite byte scan rejects malformed WIT rather than asserting over caller input."
)]
pub(super) fn cursor(
    bytes: &[u8],
    limits: crate::Limits,
) -> Result<super::Cursor<'_>, super::super::Error> {
    let byte_count = match u32::try_from(bytes.len()) {
        Ok(count) => count,
        Err(_) => return Err(super::super::exhausted()),
    };
    if bytes.len() > MAX_BYTES || byte_count > limits.bytes {
        return Err(super::super::exhausted());
    }
    let text = match core::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return Err(super::super::invalid("WIT is not UTF-8")),
    };
    let mut tokens = alloc::vec::Vec::with_capacity(bytes.len().min(MAX_TOKENS));
    let mut at = 0usize;
    let mut failure = None;
    while at < bytes.len() {
        match token(text, at, tokens.len(), limits.nodes) {
            Ok((next, token)) => {
                if let Some(token) = token {
                    tokens.push(token);
                }
                at = next;
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(super::Cursor {
            source: text,
            tokens,
            at: 0,
            remaining: limits.work,
        }),
    }
}

#[expect(
    tigerstyle::ambiguous_params,
    tigerstyle::missing_const_fn,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the scanner's private call names the byte offset separately from the admitted token count; it performs checked UTF-8 slicing and quota checks with allocating diagnostics instead of panicking on WIT input."
)]
fn token(
    text: &str,
    at: usize,
    count: usize,
    node_count: u32,
) -> Result<(usize, Option<core::ops::Range<usize>>), super::super::Error> {
    let bytes = text.as_bytes();
    let next = attempt!(scan(bytes, at));
    if next == at {
        return Err(super::super::invalid("invalid WIT token"));
    }
    if bytes[at].is_ascii_whitespace()
        || bytes.get(at..at.saturating_add(2)) == Some(b"//")
        || bytes.get(at..at.saturating_add(2)) == Some(b"/*")
    {
        return Ok((next, None));
    }
    let nodes = match u32::try_from(count) {
        Ok(count) => count,
        Err(_) => return Err(super::super::exhausted()),
    };
    if count >= MAX_TOKENS || nodes >= node_count {
        return Err(super::super::exhausted());
    }
    if text.get(at..next).is_none() {
        return Err(super::super::invalid("invalid WIT token boundary"));
    }
    Ok((next, Some(at..next)))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each accepted branch advances within the immutable byte slice, while unavailable bytes, unsupported punctuation and incomplete comments return owned WIT diagnostics."
)]
fn scan(bytes: &[u8], at: usize) -> Result<usize, super::super::Error> {
    let byte = match bytes.get(at) {
        Some(byte) => *byte,
        None => return Err(super::super::invalid("unexpected end of WIT")),
    };
    if byte.is_ascii_whitespace() {
        return Ok(at.saturating_add(1));
    }
    if bytes.get(at..at.saturating_add(2)) == Some(b"//") {
        let mut end = at.saturating_add(2);
        while end < bytes.len() && bytes[end] != b'\n' {
            end = end.saturating_add(1);
        }
        return Ok(end);
    }
    if bytes.get(at..at.saturating_add(2)) == Some(b"/*") {
        return comment(bytes, at.saturating_add(2));
    }
    if bytes.get(at..at.saturating_add(2)) == Some(b"->") {
        return Ok(at.saturating_add(2));
    }
    if b";:@{}()<>,./".contains(&byte) {
        return Ok(at.saturating_add(1));
    }
    if byte.is_ascii_alphanumeric() || byte == b'%' {
        let mut end = at.saturating_add(1);
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
            end = end.saturating_add(1);
        }
        return Ok(end);
    }
    Err(super::super::unsupported("unsupported WIT token"))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; this byte-bounded scan checks nested comment depth and termination, reporting exhaustion or malformed input instead of asserting over caller bytes."
)]
fn comment(bytes: &[u8], mut at: usize) -> Result<usize, super::super::Error> {
    let mut depth = 1u32;
    while at < bytes.len() && depth != 0 && depth <= MAX_COMMENT_DEPTH {
        let first = bytes.get(at).copied();
        let second = bytes.get(at.saturating_add(1)).copied();
        if first == Some(b'/') && second == Some(b'*') {
            depth = depth.saturating_add(1);
            at = at.saturating_add(2);
        } else if first == Some(b'*') && second == Some(b'/') {
            depth = depth.saturating_sub(1);
            at = at.saturating_add(2);
        } else {
            at = at.saturating_add(1);
        }
    }
    if depth == 0 {
        Ok(at)
    } else if depth > MAX_COMMENT_DEPTH {
        Err(super::super::exhausted())
    } else {
        Err(super::super::invalid("unterminated WIT comment"))
    }
}
