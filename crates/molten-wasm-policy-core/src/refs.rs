//! BLAKE3 content references over domain-separated policy renderings.

pub const BLAKE3_REF_PREFIX: &str = "blake3:";
pub const BLAKE3_HEX_LENGTH: usize = 64;
pub const CONTENT_REF_LENGTH: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_LENGTH;

pub fn content_ref(bytes: &[u8]) -> alloc::string::String {
    alloc::format!("{BLAKE3_REF_PREFIX}{}", blake3::hash(bytes).to_hex())
}

pub fn valid_content_ref(value: &str) -> bool {
    value.len() == CONTENT_REF_LENGTH
        && value.starts_with(BLAKE3_REF_PREFIX)
        && value[BLAKE3_REF_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn valid_ref_collection(values: &[alloc::string::String]) -> bool {
    !values.is_empty() && values.iter().all(|value| valid_content_ref(value)) && sorted_unique(values) == values
}

pub fn sorted_unique(values: &[alloc::string::String]) -> alloc::vec::Vec<alloc::string::String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

/// A growable blocker collection; validators push findings into any sink rather than a `Vec`.
pub trait VecSink<T> {
    fn push(&mut self, value: T);
}

impl<T> VecSink<T> for alloc::vec::Vec<T> {
    fn push(&mut self, value: T) {
        alloc::vec::Vec::push(self, value);
    }
}
