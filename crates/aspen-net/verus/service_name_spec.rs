//! Verus specifications for service name validation.
//!
//! Proves the byte-level predicate implemented by `src/verified/service_name.rs`:
//! valid service names match `^[a-z0-9][a-z0-9.-]{0,252}$`.

use vstd::prelude::*;

verus! {

// ========================================================================
// Spec Functions (mathematical definitions)
// ========================================================================

/// Maximum service name length accepted by Aspen mesh naming rules.
pub const MAX_SERVICE_NAME_LEN: usize = 253;
pub const ASCII_LOWER_A: u8 = 97;
pub const ASCII_LOWER_Z: u8 = 122;
pub const ASCII_DIGIT_0: u8 = 48;
pub const ASCII_DIGIT_9: u8 = 57;
pub const ASCII_HYPHEN: u8 = 45;
pub const ASCII_DOT: u8 = 46;

/// Spec: byte is ASCII lowercase `a..z`.
pub open spec fn is_ascii_lowercase_spec(byte: u8) -> bool {
    ASCII_LOWER_A <= byte && byte <= ASCII_LOWER_Z
}

/// Spec: byte is ASCII digit `0..9`.
pub open spec fn is_ascii_digit_spec(byte: u8) -> bool {
    ASCII_DIGIT_0 <= byte && byte <= ASCII_DIGIT_9
}

/// Spec: first service-name byte is lowercase ASCII or digit.
pub open spec fn is_service_name_first_byte(byte: u8) -> bool {
    is_ascii_lowercase_spec(byte) || is_ascii_digit_spec(byte)
}

/// Spec: any service-name byte is lowercase ASCII, digit, hyphen, or dot.
pub open spec fn is_service_name_body_byte(byte: u8) -> bool {
    is_service_name_first_byte(byte) || byte == ASCII_HYPHEN || byte == ASCII_DOT
}

/// Spec: byte sequence matches `^[a-z0-9][a-z0-9.-]{0,252}$`.
pub open spec fn service_name_bytes_valid(bytes: Seq<u8>) -> bool {
    1 <= bytes.len() <= MAX_SERVICE_NAME_LEN as int
        && is_service_name_first_byte(bytes[0])
        && forall|index: int| #![auto]
            0 <= index < bytes.len() ==> is_service_name_body_byte(bytes[index])
}

// ========================================================================
// Exec Functions (verified implementations)
// ========================================================================

/// SVCNAME-1..4: byte-level service name validation.
///
/// This mirrors `aspen_net::verified::service_name::is_valid_service_name`
/// after Rust's `str::as_bytes()` projection.
pub fn is_valid_service_name_bytes(bytes: &[u8]) -> (result: bool)
    ensures result == service_name_bytes_valid(bytes@)
{
    if bytes.len() == 0 || bytes.len() > MAX_SERVICE_NAME_LEN {
        assert(!service_name_bytes_valid(bytes@));
        return false;
    }

    let first = bytes[0];
    if !((ASCII_LOWER_A <= first && first <= ASCII_LOWER_Z)
        || (ASCII_DIGIT_0 <= first && first <= ASCII_DIGIT_9)) {
        assert(!is_service_name_first_byte(bytes@[0]));
        assert(!service_name_bytes_valid(bytes@));
        return false;
    }

    let mut index: usize = 0;
    while index < bytes.len()
        invariant
            1 <= bytes@.len() <= MAX_SERVICE_NAME_LEN as int,
            is_service_name_first_byte(bytes@[0]),
            0 <= index <= bytes.len(),
            forall|seen: int| #![auto]
                0 <= seen < index ==> is_service_name_body_byte(bytes@[seen]),
        decreases bytes.len() - index
    {
        let byte = bytes[index];
        if !((ASCII_LOWER_A <= byte && byte <= ASCII_LOWER_Z)
            || (ASCII_DIGIT_0 <= byte && byte <= ASCII_DIGIT_9)
            || byte == ASCII_HYPHEN
            || byte == ASCII_DOT)
        {
            assert(!is_service_name_body_byte(bytes@[index as int]));
            assert(!service_name_bytes_valid(bytes@));
            return false;
        }
        index += 1;
    }

    assert(forall|seen: int| #![auto]
        0 <= seen < bytes@.len() ==> is_service_name_body_byte(bytes@[seen]));
    true
}

// ========================================================================
// Proofs
// ========================================================================

/// SVCNAME-1: empty service names are rejected.
pub proof fn empty_service_name_rejected()
    ensures !service_name_bytes_valid(Seq::<u8>::empty())
{
}

/// SVCNAME-1: service names longer than 253 bytes are rejected.
pub proof fn overlong_service_name_rejected(bytes: Seq<u8>)
    requires bytes.len() > MAX_SERVICE_NAME_LEN as int
    ensures !service_name_bytes_valid(bytes)
{
}

/// SVCNAME-2: valid names start with lowercase ASCII or digit.
pub proof fn valid_service_name_has_valid_first_byte(bytes: Seq<u8>)
    requires service_name_bytes_valid(bytes)
    ensures is_service_name_first_byte(bytes[0])
{
}

/// SVCNAME-1: valid names are non-empty and bounded to 253 bytes.
pub proof fn valid_service_name_length_bounded(bytes: Seq<u8>)
    requires service_name_bytes_valid(bytes)
    ensures 1 <= bytes.len() <= MAX_SERVICE_NAME_LEN as int
{
}

/// SVCNAME-3: every byte in a valid name is admitted by the body predicate.
pub proof fn valid_service_name_body_bytes_admitted(bytes: Seq<u8>, index: int)
    requires
        service_name_bytes_valid(bytes),
        0 <= index < bytes.len(),
    ensures is_service_name_body_byte(bytes[index])
{
}

} // verus!
