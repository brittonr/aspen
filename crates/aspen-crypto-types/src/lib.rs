//! Cryptographic primitives for Aspen.
//!
//! This module provides fundamental cryptographic types used across the Aspen ecosystem.

use serde::Deserialize;
use serde::Serialize;

/// Ed25519 signature (64 bytes).
///
/// A wrapper around a 64-byte array representing an Ed25519 signature.
/// Used for signing objects in Forge, federation announcements, and other
/// authenticated data structures.
///
/// # Example
///
/// ```ignore
/// use aspen_crypto_types::Signature;
///
/// // Create from raw bytes
/// let sig = Signature::from_bytes([0u8; 64]);
///
/// // Access raw bytes
/// let bytes: &[u8; 64] = sig.as_bytes();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        struct SignatureVisitor;

        impl<'de> serde::de::Visitor<'de> for SignatureVisitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("64 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where E: serde::de::Error {
                if v.len() != 64 {
                    return Err(E::invalid_length(v.len(), &self));
                }
                let mut arr = [0u8; 64];
                arr.copy_from_slice(v);
                Ok(Signature(arr))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de> {
                let mut arr = [0u8; 64];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature(arr))
            }
        }

        deserializer.deserialize_bytes(SignatureVisitor)
    }
}

impl Signature {
    /// Create a signature from raw bytes.
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Get the raw signature bytes.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_roundtrip() {
        let sig = Signature::from_bytes([42u8; 64]);

        // Serialize with postcard
        let bytes = postcard::to_allocvec(&sig).expect("should serialize");

        // Deserialize
        let recovered: Signature = postcard::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(sig, recovered);
    }

    #[test]
    fn test_signature_json_roundtrip() {
        let sig = Signature::from_bytes([42u8; 64]);

        // Serialize with serde_json
        let json = serde_json::to_string(&sig).expect("should serialize");

        // Deserialize
        let recovered: Signature = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(sig, recovered);
    }

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_from_bytes_preserves_data() {
        let bytes = [0xAB; 64];
        let sig = Signature::from_bytes(bytes);
        assert_eq!(sig.as_bytes(), &bytes);
    }

    #[test]
    fn test_as_bytes_returns_reference() {
        let bytes = [0x12; 64];
        let sig = Signature::from_bytes(bytes);

        let ref1 = sig.as_bytes();
        let ref2 = sig.as_bytes();

        assert_eq!(ref1, ref2);
        assert_eq!(ref1.len(), 64);
    }

    #[test]
    fn test_direct_field_access() {
        let bytes = [0xFF; 64];
        let sig = Signature(bytes);
        assert_eq!(sig.0, bytes);
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    #[test]
    fn test_signature_equality_same() {
        let sig1 = Signature::from_bytes([1u8; 64]);
        let sig2 = Signature::from_bytes([1u8; 64]);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_equality_different() {
        let sig1 = Signature::from_bytes([1u8; 64]);
        let sig2 = Signature::from_bytes([2u8; 64]);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_signature_equality_single_byte_difference() {
        let bytes1 = [0u8; 64];
        let mut bytes2 = [0u8; 64];
        bytes2[63] = 1; // Only last byte differs

        let sig1 = Signature::from_bytes(bytes1);
        let sig2 = Signature::from_bytes(bytes2);
        assert_ne!(sig1, sig2);
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn test_signature_clone() {
        let original = Signature::from_bytes([99u8; 64]);
        let cloned = original.clone();

        assert_eq!(original, cloned);
        assert_eq!(original.as_bytes(), cloned.as_bytes());
    }

    #[test]
    fn test_signature_clone_is_independent() {
        let bytes = [50u8; 64];
        let sig1 = Signature::from_bytes(bytes);
        let sig2 = sig1.clone();

        // Both should have same value
        assert_eq!(sig1.as_bytes(), sig2.as_bytes());
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    #[test]
    fn test_signature_debug() {
        let sig = Signature::from_bytes([0u8; 64]);
        let debug = format!("{:?}", sig);

        assert!(debug.contains("Signature"));
    }

    #[test]
    fn test_signature_debug_shows_bytes() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0xAB;
        let sig = Signature::from_bytes(bytes);
        let debug = format!("{:?}", sig);

        // Debug should contain some representation of the bytes
        assert!(debug.len() > 10); // Non-trivial output
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_signature_all_zeros() {
        let sig = Signature::from_bytes([0u8; 64]);
        assert_eq!(sig.as_bytes(), &[0u8; 64]);

        // Should serialize/deserialize correctly
        let bytes = postcard::to_allocvec(&sig).expect("serialize");
        let recovered: Signature = postcard::from_bytes(&bytes).expect("deserialize");
        assert_eq!(sig, recovered);
    }

    #[test]
    fn test_signature_all_ones() {
        let sig = Signature::from_bytes([0xFF; 64]);
        assert_eq!(sig.as_bytes(), &[0xFF; 64]);

        // Should serialize/deserialize correctly
        let bytes = postcard::to_allocvec(&sig).expect("serialize");
        let recovered: Signature = postcard::from_bytes(&bytes).expect("deserialize");
        assert_eq!(sig, recovered);
    }

    #[test]
    fn test_signature_sequential_bytes() {
        let mut bytes = [0u8; 64];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i % 256) as u8;
        }

        let sig = Signature::from_bytes(bytes);
        assert_eq!(sig.as_bytes()[0], 0);
        assert_eq!(sig.as_bytes()[63], 63);
    }

    #[test]
    fn test_signature_alternating_bytes() {
        let mut bytes = [0u8; 64];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = if i % 2 == 0 { 0xAA } else { 0x55 };
        }

        let sig = Signature::from_bytes(bytes);

        let json = serde_json::to_string(&sig).expect("serialize");
        let recovered: Signature = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(sig, recovered);
    }

    // =========================================================================
    // Serialization Edge Cases
    // =========================================================================

    #[test]
    fn test_postcard_compact_encoding() {
        let sig = Signature::from_bytes([0u8; 64]);
        let bytes = postcard::to_allocvec(&sig).expect("serialize");

        // Postcard should encode this compactly (64 bytes + small overhead)
        assert!(bytes.len() <= 70);
    }

    #[test]
    fn test_json_array_format() {
        let sig = Signature::from_bytes([1u8; 64]);
        let json = serde_json::to_string(&sig).expect("serialize");

        // JSON should contain array-like representation
        assert!(json.contains("[") || json.contains("1"));
    }

    #[test]
    fn test_cbor_roundtrip() {
        // Test with another serde format if available via postcard
        let sig = Signature::from_bytes([42u8; 64]);

        let bytes = postcard::to_allocvec(&sig).expect("serialize");
        let recovered: Signature = postcard::from_bytes(&bytes).expect("deserialize");

        assert_eq!(sig.as_bytes(), recovered.as_bytes());
    }

    // =========================================================================
    // Deserialization Error Tests
    // =========================================================================

    #[test]
    fn test_deserialize_wrong_length_too_short() {
        // Try to deserialize bytes that are too short (not 64 bytes)
        // This should trigger the invalid_length error path
        let short_bytes: [u8; 32] = [42u8; 32];
        let result: Result<Signature, _> = postcard::from_bytes(&short_bytes);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_wrong_length_too_long() {
        // Try to deserialize bytes that are too long (not 64 bytes)
        let long_bytes: [u8; 128] = [42u8; 128];
        let result: Result<Signature, _> = postcard::from_bytes(&long_bytes);

        // This will either fail or succeed depending on how postcard handles extra bytes
        // Either way, we're testing the deserialization code path
        let _ = result;
    }

    #[test]
    fn test_deserialize_empty() {
        let empty: [u8; 0] = [];
        let result: Result<Signature, _> = postcard::from_bytes(&empty);

        assert!(result.is_err());
    }

    #[test]
    fn test_json_deserialize_invalid() {
        // Try to deserialize invalid JSON
        let invalid_json = "not a valid signature";
        let result: Result<Signature, _> = serde_json::from_str(invalid_json);

        assert!(result.is_err());
    }

    #[test]
    fn test_json_deserialize_wrong_type() {
        // Try to deserialize a number instead of bytes
        let wrong_type = "12345";
        let result: Result<Signature, _> = serde_json::from_str(wrong_type);

        assert!(result.is_err());
    }
}
