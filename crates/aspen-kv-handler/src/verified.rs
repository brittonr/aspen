//! Pure functions for KV handler logic.
//!
//! This module contains pure functions for deterministic behavior.

/// Convert bytes to a UTF-8 string, replacing invalid sequences with the replacement character.
///
/// This is used when client values may contain arbitrary bytes but we need to store
/// them as strings internally. Invalid UTF-8 sequences are replaced with U+FFFD.
///
/// # Arguments
/// * `bytes` - The byte slice to convert
///
/// # Returns
/// A String containing the UTF-8 representation
#[inline]
pub fn bytes_to_string_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_utf8() {
        let bytes = b"hello world";
        assert_eq!(bytes_to_string_lossy(bytes), "hello world");
    }

    #[test]
    fn test_empty_bytes() {
        let bytes: &[u8] = b"";
        assert_eq!(bytes_to_string_lossy(bytes), "");
    }

    #[test]
    fn test_unicode() {
        let s = "Hello, 世界! 🌍";
        assert_eq!(bytes_to_string_lossy(s.as_bytes()), s);
    }

    #[test]
    fn test_invalid_utf8() {
        // Invalid UTF-8 sequence
        let bytes: &[u8] = &[0x80, 0x81, 0x82];
        let result = bytes_to_string_lossy(bytes);
        // Each invalid byte should be replaced with U+FFFD
        assert!(result.contains('\u{FFFD}'));
    }
}
