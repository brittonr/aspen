//! Pure conversion functions for RPC types.
//!
//! These functions handle conversions between internal types and wire format
//! types used in client RPC responses.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - No allocations in simple conversions

/// Convert bytes to a String using lossy UTF-8 conversion.
///
/// This is a pure wrapper around `String::from_utf8_lossy` that owns
/// the result. Invalid UTF-8 sequences are replaced with the Unicode
/// replacement character (U+FFFD).
///
/// # Arguments
///
/// * `bytes` - Byte slice to convert
///
/// # Returns
///
/// A String containing the UTF-8 representation of the bytes.
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::bytes_to_string_lossy;
///
/// assert_eq!(bytes_to_string_lossy(b"hello"), "hello");
/// assert_eq!(bytes_to_string_lossy(&[0xFF, 0xFE]), "\u{FFFD}\u{FFFD}");
/// ```
#[inline]
pub fn bytes_to_string_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Convert an optional byte slice to an optional String using lossy UTF-8.
///
/// # Arguments
///
/// * `bytes` - Optional byte slice to convert
///
/// # Returns
///
/// - `None` if input is `None`
/// - `Some(String)` with the UTF-8 representation if input is `Some`
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::optional_bytes_to_string_lossy;
///
/// assert_eq!(optional_bytes_to_string_lossy(None), None);
/// assert_eq!(optional_bytes_to_string_lossy(Some(&b"test"[..])), Some("test".to_string()));
/// ```
#[inline]
pub fn optional_bytes_to_string_lossy(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(bytes_to_string_lossy)
}

/// DLQ reason string constants.
///
/// These match the string representations expected by clients.
pub mod dlq_reason {
    /// Reason: Item exceeded maximum delivery attempts.
    pub const MAX_DELIVERY_ATTEMPTS: &str = "max_delivery_attempts";

    /// Reason: Item was explicitly rejected via nack with move_to_dlq=true.
    pub const EXPLICITLY_REJECTED: &str = "explicitly_rejected";

    /// Reason: Item expired while still pending in the queue.
    pub const EXPIRED_WHILE_PENDING: &str = "expired_while_pending";
}

/// Convert a DLQ reason enum variant to its string representation.
///
/// This is used when converting internal DLQReason enum values to
/// string representations for the client RPC response.
///
/// # Arguments
///
/// * `reason` - The reason variant (0 = MaxDeliveryAttemptsExceeded, 1 = ExplicitlyRejected, 2 =
///   ExpiredWhilePending)
///
/// # Returns
///
/// A static string slice representing the reason.
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::convert_dlq_reason_index;
///
/// assert_eq!(convert_dlq_reason_index(0), "max_delivery_attempts");
/// assert_eq!(convert_dlq_reason_index(1), "explicitly_rejected");
/// assert_eq!(convert_dlq_reason_index(2), "expired_while_pending");
/// assert_eq!(convert_dlq_reason_index(99), "unknown");
/// ```
#[inline]
pub const fn convert_dlq_reason_index(reason: u8) -> &'static str {
    match reason {
        0 => dlq_reason::MAX_DELIVERY_ATTEMPTS,
        1 => dlq_reason::EXPLICITLY_REJECTED,
        2 => dlq_reason::EXPIRED_WHILE_PENDING,
        _ => "unknown",
    }
}

/// Dequeued item data extracted for response building.
///
/// This struct contains all the fields needed to build a
/// QueueDequeuedItemResponse, extracted from the coordination layer type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DequeuedItemData {
    /// Unique identifier for the item.
    pub item_id: u64,
    /// The item payload.
    pub payload: String,
    /// Receipt handle for acknowledging the item.
    pub receipt_handle: String,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
    /// When the item was enqueued (ms since epoch).
    pub enqueued_at_ms: u64,
    /// When visibility timeout expires (ms since epoch).
    pub visibility_deadline_ms: u64,
}

/// Queue item data extracted for response building (peek operation).
///
/// This struct contains all the fields needed to build a
/// QueueItemResponse, extracted from the coordination layer type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItemData {
    /// Unique identifier for the item.
    pub item_id: u64,
    /// The item payload.
    pub payload: String,
    /// When the item was enqueued (ms since epoch).
    pub enqueued_at_ms: u64,
    /// When the item expires (ms since epoch), if set.
    pub expires_at_ms: Option<u64>,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
}

/// DLQ item data extracted for response building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DLQItemData {
    /// Unique identifier for the item.
    pub item_id: u64,
    /// The item payload.
    pub payload: String,
    /// When the item was enqueued (ms since epoch).
    pub enqueued_at_ms: u64,
    /// Number of delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason the item was moved to DLQ.
    pub reason: String,
    /// When the item was moved to DLQ (ms since epoch).
    pub moved_at_ms: u64,
    /// Last error message if any.
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_string_lossy_valid_utf8() {
        assert_eq!(bytes_to_string_lossy(b"hello"), "hello");
        assert_eq!(bytes_to_string_lossy(b""), "");
        assert_eq!(bytes_to_string_lossy("unicode: \u{1F600}".as_bytes()), "unicode: \u{1F600}");
    }

    #[test]
    fn test_bytes_to_string_lossy_invalid_utf8() {
        // Invalid UTF-8 bytes get replaced with U+FFFD
        let result = bytes_to_string_lossy(&[0xFF, 0xFE]);
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn test_optional_bytes_to_string_lossy_none() {
        assert_eq!(optional_bytes_to_string_lossy(None), None);
    }

    #[test]
    fn test_optional_bytes_to_string_lossy_some() {
        assert_eq!(optional_bytes_to_string_lossy(Some(b"test")), Some("test".to_string()));
    }

    #[test]
    fn test_convert_dlq_reason_known() {
        assert_eq!(convert_dlq_reason_index(0), "max_delivery_attempts");
        assert_eq!(convert_dlq_reason_index(1), "explicitly_rejected");
        assert_eq!(convert_dlq_reason_index(2), "expired_while_pending");
    }

    #[test]
    fn test_convert_dlq_reason_unknown() {
        assert_eq!(convert_dlq_reason_index(3), "unknown");
        assert_eq!(convert_dlq_reason_index(255), "unknown");
    }
}
