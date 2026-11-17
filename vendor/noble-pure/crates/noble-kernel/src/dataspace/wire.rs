use super::*;

/// The only admitted Preserves value is the canonical text record
/// `<service "NAME" #t>` or `<service "NAME" #f>`. No embedded values, general
/// constructors, escaping or extension tags belong to this versioned schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceRef<'a> {
    pub name: &'a str,
    pub ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Packet {
    bytes: [u8; MAX_WIRE_BYTES],
    len: usize,
}

impl Packet {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

pub fn valid_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return false;
    }
    let bytes = name.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !byte.is_ascii_alphanumeric() && byte != b'_' && byte != b'-' {
            return false;
        }
        index += 1;
    }
    true
}

pub fn encode_service(name: &str, ready: bool) -> Result<Packet, Error> {
    // Both admitted record sizes must fit the wire budget, including the
    // longest valid name, even if the configured limits change later.
    const MIN_SERVICE_BYTES: usize = b"<service \"\" #t>".len();
    const {
        assert!(MIN_SERVICE_BYTES <= MAX_WIRE_BYTES);
        assert!(MAX_NAME_BYTES <= MAX_WIRE_BYTES.saturating_sub(MIN_SERVICE_BYTES));
    }
    if !valid_name(name) {
        return Err(Error::InvalidName);
    }
    let mut bytes = [0u8; MAX_WIRE_BYTES];
    let prefix = b"<service \"";
    let suffix = if ready { b"\" #t>" } else { b"\" #f>" };
    let Some(end) = prefix.len().checked_add(name.len()) else {
        return Err(Error::ByteLimit);
    };
    let Some(total) = end.checked_add(suffix.len()) else {
        return Err(Error::ByteLimit);
    };
    if total > MAX_WIRE_BYTES {
        return Err(Error::ByteLimit);
    }
    bytes[..prefix.len()].copy_from_slice(prefix);
    bytes[prefix.len()..end].copy_from_slice(name.as_bytes());
    bytes[end..total].copy_from_slice(suffix);
    Ok(Packet { bytes, len: total })
}

fn check_depth(bytes: &[u8], max_depth: u32) -> Result<(), Error> {
    // Depth inspection is bounded by the byte budget, and rejects a depth-5
    // input before the more specific closed-schema check. Quoted names cannot
    // contain brackets or escapes under the admitted service-name alphabet.
    let mut depth = 0u32;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'<' {
            depth += 1;
            if depth > max_depth {
                return Err(Error::DepthLimit);
            }
        } else if byte == b'>' {
            depth = depth.saturating_sub(1);
        }
        index += 1;
    }
    Ok(())
}

fn has_service_prefix(bytes: &[u8]) -> bool {
    let prefix = b"<service \"";
    let mut index = 0;
    while index < prefix.len() {
        if bytes[index] != prefix[index] {
            return false;
        }
        index += 1;
    }
    true
}

pub fn decode_service(bytes: &[u8], max_depth: u32) -> Result<ServiceRef<'_>, Error> {
    // These are schema-budget invariants: future limit changes must not make
    // the minimum record or any admitted service name unrepresentable.
    const MIN_SERVICE_BYTES: usize = b"<service \"\" #t>".len();
    const {
        assert!(MIN_SERVICE_BYTES <= MAX_WIRE_BYTES);
        assert!(MAX_NAME_BYTES <= MAX_WIRE_BYTES.saturating_sub(MIN_SERVICE_BYTES));
    }
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(Error::ByteLimit);
    }
    if max_depth == 0 || max_depth > MAX_WIRE_DEPTH {
        return Err(Error::DepthLimit);
    }
    attempt!(check_depth(bytes, max_depth));
    let prefix_len = b"<service \"".len();
    let suffix_len = b"\" #t>".len();
    if bytes.len() < MIN_SERVICE_BYTES || !has_service_prefix(bytes) {
        return Err(Error::Schema);
    }
    let Some(name_end) = bytes.len().checked_sub(suffix_len) else {
        return Err(Error::Schema);
    };
    if name_end < prefix_len {
        return Err(Error::Schema);
    }
    let suffix = &bytes[name_end..];
    let has_quote_and_space = suffix[0] == b'"' && suffix[1] == b' ';
    let has_tag_and_close = suffix[2] == b'#' && suffix[4] == b'>';
    if !has_quote_and_space || !has_tag_and_close {
        return Err(Error::Schema);
    }
    let is_ready = match suffix[3] {
        b't' => true,
        b'f' => false,
        _ => return Err(Error::Schema),
    };
    let name = &bytes[prefix_len..name_end];
    let name = match core::str::from_utf8(name) {
        Ok(name) => name,
        Err(_) => return Err(Error::Schema),
    };
    if !valid_name(name) {
        return Err(Error::Schema);
    }
    Ok(ServiceRef {
        name,
        ready: is_ready,
    })
}
