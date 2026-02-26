//! SHA-1 hash type for Git interoperability.
//!
//! Standard Git uses SHA-1 (20 bytes) for content addressing.
//! This module provides a wrapper type with conversion utilities.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use super::error::BridgeError;

/// SHA-1 hash (20 bytes) as used by standard Git.
///
/// This type wraps a 20-byte array and provides conversion utilities
/// for hex encoding/decoding and display.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha1Hash(pub [u8; 20]);

impl Sha1Hash {
    /// Size of a SHA-1 hash in bytes.
    pub const SIZE: usize = 20;

    /// Size of a SHA-1 hash as a hex string.
    pub const HEX_SIZE: usize = 40;

    /// Create a SHA-1 hash from a byte array.
    #[inline]
    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Create a SHA-1 hash from a byte slice.
    ///
    /// Returns an error if the slice is not exactly 20 bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, BridgeError> {
        if bytes.len() != Self::SIZE {
            return Err(BridgeError::InvalidHashLength {
                expected: Self::SIZE as u32,
                actual: bytes.len() as u32,
            });
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }

    /// Create a SHA-1 hash from a hex string.
    ///
    /// Returns an error if the string is not exactly 40 hex characters.
    pub fn from_hex(s: &str) -> Result<Self, BridgeError> {
        if s.len() != Self::HEX_SIZE {
            return Err(BridgeError::InvalidHashLength {
                expected: Self::HEX_SIZE as u32,
                actual: s.len() as u32,
            });
        }

        let bytes = hex::decode(s).map_err(|e| BridgeError::InvalidHexEncoding { message: e.to_string() })?;

        Self::from_slice(&bytes)
    }

    /// Convert the hash to a hex string.
    #[inline]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Get the underlying byte array.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Get the underlying bytes as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Create a zero hash (all zeros).
    ///
    /// This is used in Git to represent "no object" in certain contexts.
    #[inline]
    pub const fn zero() -> Self {
        Self([0u8; 20])
    }

    /// Check if this is a zero hash.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 20]
    }
}

impl fmt::Debug for Sha1Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha1Hash({})", self.to_hex())
    }
}

impl fmt::Display for Sha1Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Sha1Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 20]> for Sha1Hash {
    fn from(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for Sha1Hash {
    type Error = BridgeError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_slice(bytes)
    }
}

impl TryFrom<&str> for Sha1Hash {
    type Error = BridgeError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_hex(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_from_hex() {
        let hex = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
        let hash = Sha1Hash::from_hex(hex).unwrap();
        assert_eq!(hash.to_hex(), hex);
    }

    #[test]
    fn test_sha1_from_hex_invalid_length() {
        let result = Sha1Hash::from_hex("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_sha1_from_hex_invalid_chars() {
        let result = Sha1Hash::from_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_sha1_zero() {
        let zero = Sha1Hash::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.to_hex(), "0000000000000000000000000000000000000000");
    }

    #[test]
    fn test_sha1_display() {
        let hash = Sha1Hash::from_bytes([0xab; 20]);
        assert_eq!(format!("{hash}"), "abababababababababababababababababababab");
    }

    #[test]
    fn test_sha1_from_slice() {
        let bytes = [0x12u8; 20];
        let hash = Sha1Hash::from_slice(&bytes).unwrap();
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_sha1_from_slice_wrong_length() {
        let result = Sha1Hash::from_slice(&[0u8; 19]);
        assert!(result.is_err());
    }
}
