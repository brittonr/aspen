// Canonical Preserves values for protocol projections.
//
// The subset the protocol oracle needs: signed integers, strings, byte
// strings, symbols, records with symbol labels, and sequences. The encoding is
// the canonical Preserves packed binary form: integers are minimal big-endian
// two's complement, lengths are base-128 varints, and records and sequences
// end with the end tag. No sets or dictionaries are produced, so no ordering
// rule beyond construction order applies; callers sort where order matters.

/// One canonical value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalValue {
    Integer(i128),
    String(String),
    Bytes(Vec<u8>),
    Symbol(String),
    Record { label: String, fields: Vec<CanonicalValue> },
    Sequence(Vec<CanonicalValue>),
}

const TAG_END: u8 = 0x84;
const TAG_SIGNED_INTEGER: u8 = 0xb0;
const TAG_STRING: u8 = 0xb1;
const TAG_BYTE_STRING: u8 = 0xb2;
const TAG_SYMBOL: u8 = 0xb3;
const TAG_RECORD: u8 = 0xb4;
const TAG_SEQUENCE: u8 = 0xb5;
const VARINT_PAYLOAD_BITS: u32 = 7;
const VARINT_CONTINUATION: u8 = 0x80;
const VARINT_PAYLOAD_MASK: u8 = 0x7f;

impl CanonicalValue {
    pub fn symbol(value: &str) -> Self {
        Self::Symbol(value.to_owned())
    }

    pub fn string(value: &str) -> Self {
        Self::String(value.to_owned())
    }

    pub fn unsigned(value: u64) -> Self {
        Self::Integer(i128::from(value))
    }

    pub fn record(label: &str, fields: Vec<Self>) -> Self {
        Self::Record {
            label: label.to_owned(),
            fields,
        }
    }

    /// An optional value: `<none>` or `<some value>`.
    pub fn optional(value: Option<Self>) -> Self {
        match value {
            None => Self::record("none", Vec::new()),
            Some(value) => Self::record("some", vec![value]),
        }
    }
}

/// Encode a value in canonical Preserves packed binary form.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn encode_canonical(value: &CanonicalValue) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    write_value(&mut bytes, value);
    bytes
}

/// A domain-separated BLAKE3 ref over canonical bytes.
pub fn domain_ref(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(bytes);
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// Whether a value is a `blake3:` ref with 64 lowercase hexadecimal digits.
pub fn is_content_ref(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn write_value(bytes: &mut Vec<u8>, value: &CanonicalValue) {
    match value {
        CanonicalValue::Integer(integer) => write_atom(bytes, TAG_SIGNED_INTEGER, &minimal_twos_complement(*integer)),
        CanonicalValue::String(text) => write_atom(bytes, TAG_STRING, text.as_bytes()),
        CanonicalValue::Bytes(raw) => write_atom(bytes, TAG_BYTE_STRING, raw),
        CanonicalValue::Symbol(name) => write_atom(bytes, TAG_SYMBOL, name.as_bytes()),
        CanonicalValue::Record { label, fields } => {
            bytes.push(TAG_RECORD);
            write_atom(bytes, TAG_SYMBOL, label.as_bytes());
            for field in fields {
                write_value(bytes, field);
            }
            bytes.push(TAG_END);
        }
        CanonicalValue::Sequence(items) => {
            bytes.push(TAG_SEQUENCE);
            for item in items {
                write_value(bytes, item);
            }
            bytes.push(TAG_END);
        }
    }
}

fn write_atom(bytes: &mut Vec<u8>, tag: u8, payload: &[u8]) {
    bytes.push(tag);
    let mut length = payload.len();
    loop {
        let low = length.to_le_bytes()[0] & VARINT_PAYLOAD_MASK;
        length >>= VARINT_PAYLOAD_BITS;
        if length == 0 {
            bytes.push(low);
            break;
        }
        bytes.push(low | VARINT_CONTINUATION);
    }
    bytes.extend_from_slice(payload);
}

/// The shortest big-endian two's complement form; zero is empty.
fn minimal_twos_complement(value: i128) -> Vec<u8> {
    if value == 0 {
        return Vec::new();
    }
    let full = value.to_be_bytes();
    let mut start = 0;
    while start + 1 < full.len() {
        let (current, next) = (full[start], full[start + 1]);
        let is_redundant = (current == 0x00 && next & 0x80 == 0) || (current == 0xff && next & 0x80 != 0);
        if !is_redundant {
            break;
        }
        start += 1;
    }
    full[start..].to_vec()
}
