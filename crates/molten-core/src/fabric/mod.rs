//! Pure distributed-systems fabric contracts.
//!
//! These models contain no adapter objects or external effects. Rust values are
//! in-memory inputs to validation; the `molten` crate owns their canonical
//! Preserves projection and BLAKE3 identity.

mod boundary;
mod evidence;
mod port;
mod reference;
mod tier;

pub use boundary::*;
pub use evidence::*;
pub use port::*;
pub use reference::*;
pub use tier::*;

pub(crate) const MAX_FABRIC_COLLECTION_ITEMS: usize = 64;
pub(crate) const MAX_FABRIC_PORTS: usize = 128;
pub(crate) const MAX_FABRIC_TEXT_CHARS: usize = 256;

const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len().saturating_add(BLAKE3_HEX_CHAR_COUNT);

pub(crate) fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

pub(crate) fn valid_fabric_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FABRIC_TEXT_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    value.len() == BLAKE3_REF_CHAR_COUNT
        && hex.len() == BLAKE3_HEX_CHAR_COUNT
        && hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_blake3_ref_requires_exact_lowercase_ascii_hex() {
        let hex = "a".repeat(BLAKE3_HEX_CHAR_COUNT);
        assert!(valid_blake3_ref(&format!("{BLAKE3_REF_PREFIX}{hex}")));
        assert!(!valid_blake3_ref(&format!("{BLAKE3_REF_PREFIX}A{}", "a".repeat(63))));
        assert!(!valid_blake3_ref(&format!("{BLAKE3_REF_PREFIX}é{}", "a".repeat(62))));
        assert!(!valid_blake3_ref(&format!("sha256:{hex}")));
    }
}
