#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the focused stub preserves the published fabric token API name"
)]

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

pub(crate) fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn valid_fabric_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}
