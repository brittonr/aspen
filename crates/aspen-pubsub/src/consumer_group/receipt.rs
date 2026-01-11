//! Receipt handle generation and validation.
//!
//! Receipt handles are tamper-proof identifiers returned when a message is delivered
//! to a consumer. They are used for acknowledgment operations and include a
//! cryptographic signature to prevent forgery.
//!
//! # Format
//!
//! `{cursor}:{consumer_id}:{fencing_token}:{attempt}:{timestamp}:{signature}`
//!
//! The signature is a truncated Blake3 keyed hash of all components, making
//! handles unforgeable without knowledge of the secret.

use blake3::Hasher;

/// Components embedded in a receipt handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptHandleComponents {
    /// Message cursor (Raft log index).
    pub cursor: u64,
    /// Consumer ID who received the message.
    pub consumer_id: String,
    /// Fencing token at delivery time.
    pub fencing_token: u64,
    /// Delivery attempt number (1-based).
    pub delivery_attempt: u32,
    /// Timestamp when message was delivered (Unix ms).
    pub delivered_at_ms: u64,
}

/// Generate a tamper-proof receipt handle.
///
/// The handle includes all identifying information plus a cryptographic
/// signature that can only be verified with the same secret.
///
/// # Arguments
///
/// * `secret` - 32-byte secret key for HMAC signing
/// * `components` - The data to encode in the handle
///
/// # Returns
///
/// A colon-separated string with the format:
/// `{cursor}:{consumer_id}:{fencing_token}:{attempt}:{timestamp}:{signature}`
pub fn generate_receipt_handle(secret: &[u8; 32], components: &ReceiptHandleComponents) -> String {
    let signature = compute_signature(secret, components);

    format!(
        "{}:{}:{}:{}:{}:{}",
        components.cursor,
        components.consumer_id,
        components.fencing_token,
        components.delivery_attempt,
        components.delivered_at_ms,
        signature
    )
}

/// Parse and validate a receipt handle.
///
/// Verifies the signature matches the components, returning None if
/// the handle is malformed or has been tampered with.
///
/// # Arguments
///
/// * `secret` - 32-byte secret key used for signing
/// * `handle` - The receipt handle string to parse
///
/// # Returns
///
/// `Some(components)` if valid, `None` if malformed or signature mismatch.
pub fn parse_receipt_handle(secret: &[u8; 32], handle: &str) -> Option<ReceiptHandleComponents> {
    let parts: Vec<&str> = handle.split(':').collect();
    if parts.len() != 6 {
        return None;
    }

    let cursor = parts[0].parse().ok()?;
    let consumer_id = parts[1].to_string();
    let fencing_token = parts[2].parse().ok()?;
    let delivery_attempt = parts[3].parse().ok()?;
    let delivered_at_ms = parts[4].parse().ok()?;
    let provided_signature = parts[5];

    let components = ReceiptHandleComponents {
        cursor,
        consumer_id,
        fencing_token,
        delivery_attempt,
        delivered_at_ms,
    };

    // Recompute expected signature
    let expected_signature = compute_signature(secret, &components);

    // Constant-time comparison to prevent timing attacks
    if constant_time_eq(provided_signature.as_bytes(), expected_signature.as_bytes()) {
        Some(components)
    } else {
        None
    }
}

/// Compute the signature for receipt handle components.
fn compute_signature(secret: &[u8; 32], components: &ReceiptHandleComponents) -> String {
    let mut hasher = Hasher::new_keyed(secret);
    hasher.update(&components.cursor.to_le_bytes());
    hasher.update(components.consumer_id.as_bytes());
    hasher.update(&components.fencing_token.to_le_bytes());
    hasher.update(&components.delivery_attempt.to_le_bytes());
    hasher.update(&components.delivered_at_ms.to_le_bytes());

    let hash = hasher.finalize();
    // Use first 16 bytes (128 bits) for reasonable handle length
    hex::encode(&hash.as_bytes()[..16])
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Simple hex encoding (avoids external dependency).
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut result = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> [u8; 32] {
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ]
    }

    #[test]
    fn test_generate_and_parse_roundtrip() {
        let secret = test_secret();
        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret, &components);
        let parsed = parse_receipt_handle(&secret, &handle);

        assert_eq!(parsed, Some(components));
    }

    #[test]
    fn test_parse_malformed_handle() {
        let secret = test_secret();

        // Too few parts
        assert!(parse_receipt_handle(&secret, "123:consumer:456").is_none());

        // Too many parts
        assert!(parse_receipt_handle(&secret, "1:2:3:4:5:6:7").is_none());

        // Invalid cursor
        assert!(parse_receipt_handle(&secret, "abc:consumer:456:1:1000:sig").is_none());

        // Invalid fencing token
        assert!(parse_receipt_handle(&secret, "123:consumer:abc:1:1000:sig").is_none());
    }

    #[test]
    fn test_tampered_signature_rejected() {
        let secret = test_secret();
        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret, &components);

        // Tamper with the signature
        let parts: Vec<&str> = handle.split(':').collect();
        let tampered = format!(
            "{}:{}:{}:{}:{}:{}",
            parts[0], parts[1], parts[2], parts[3], parts[4],
            "0000000000000000000000000000000"
        );

        assert!(parse_receipt_handle(&secret, &tampered).is_none());
    }

    #[test]
    fn test_tampered_cursor_rejected() {
        let secret = test_secret();
        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret, &components);

        // Tamper with the cursor
        let parts: Vec<&str> = handle.split(':').collect();
        let tampered = format!(
            "{}:{}:{}:{}:{}:{}",
            99999, // Changed cursor
            parts[1], parts[2], parts[3], parts[4], parts[5]
        );

        assert!(parse_receipt_handle(&secret, &tampered).is_none());
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let secret1 = test_secret();
        let mut secret2 = test_secret();
        secret2[0] = 0xff; // Different secret

        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret1, &components);

        // Try to parse with different secret
        assert!(parse_receipt_handle(&secret2, &handle).is_none());
    }

    #[test]
    fn test_consumer_id_with_special_chars() {
        let secret = test_secret();
        // Consumer IDs are validated to only contain alphanumeric, hyphens, underscores
        // but we test the receipt handle works with valid characters
        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1_test".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret, &components);
        let parsed = parse_receipt_handle(&secret, &handle);

        assert_eq!(parsed, Some(components));
    }

    #[test]
    fn test_handle_format() {
        let secret = test_secret();
        let components = ReceiptHandleComponents {
            cursor: 12345,
            consumer_id: "consumer-1".to_string(),
            fencing_token: 98765,
            delivery_attempt: 1,
            delivered_at_ms: 1704067200000,
        };

        let handle = generate_receipt_handle(&secret, &components);
        let parts: Vec<&str> = handle.split(':').collect();

        assert_eq!(parts.len(), 6);
        assert_eq!(parts[0], "12345");
        assert_eq!(parts[1], "consumer-1");
        assert_eq!(parts[2], "98765");
        assert_eq!(parts[3], "1");
        assert_eq!(parts[4], "1704067200000");
        // Signature is 32 hex chars (16 bytes)
        assert_eq!(parts[5].len(), 32);
    }
}
