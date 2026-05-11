//! Verus specs for Nostr identity/key structural admission.
//!
//! Production `src/keys.rs` delegates secp256k1/Schnorr key generation,
//! signing, public-key derivation, and hex parsing to the `nostr` crate. This
//! module verifies the structural shell contract around that crypto boundary:
//! raw secret keys are fixed-width 32-byte values, persisted key material is
//! 64 hex characters, x-only public keys are 32 bytes / 64 hex characters, and
//! malformed persisted shapes are rejected before any crypto truth is claimed.

use vstd::prelude::*;

verus! {

pub open spec const SECP256K1_SECRET_BYTES: int = 32;
pub open spec const XONLY_PUBLIC_KEY_BYTES: int = 32;
pub open spec const EVENT_ID_BYTES: int = 32;
pub open spec const SCHNORR_SIGNATURE_BYTES: int = 64;
pub open spec const HEX_CHARS_PER_BYTE: int = 2;
pub open spec const SECRET_KEY_HEX_LEN: int = 64;
pub open spec const PUBLIC_KEY_HEX_LEN: int = 64;
pub open spec const EVENT_ID_HEX_LEN: int = 64;
pub open spec const SIGNATURE_HEX_LEN: int = 128;

pub enum SecretKeyAdmissionSpec {
    Accepted,
    WrongLength,
    CryptoRejected,
}

pub enum PersistedSecretAdmissionSpec {
    Accepted,
    WrongLength,
    NonHex,
    CryptoRejected,
}

pub open spec fn hex_len_for_bytes(byte_len: int) -> int {
    byte_len * HEX_CHARS_PER_BYTE
}

pub open spec fn raw_secret_shape_valid(byte_len: int) -> bool {
    byte_len == SECP256K1_SECRET_BYTES
}

pub open spec fn xonly_public_key_shape_valid(byte_len: int) -> bool {
    byte_len == XONLY_PUBLIC_KEY_BYTES
}

pub open spec fn event_id_shape_valid(byte_len: int) -> bool {
    byte_len == EVENT_ID_BYTES
}

pub open spec fn schnorr_signature_shape_valid(byte_len: int) -> bool {
    byte_len == SCHNORR_SIGNATURE_BYTES
}

pub open spec fn persisted_secret_shape_valid(hex_len: int, all_hex: bool) -> bool {
    hex_len == SECRET_KEY_HEX_LEN && all_hex
}

pub open spec fn public_key_hex_shape_valid(hex_len: int, all_hex: bool) -> bool {
    hex_len == PUBLIC_KEY_HEX_LEN && all_hex
}

pub open spec fn event_id_hex_shape_valid(hex_len: int, all_hex: bool) -> bool {
    hex_len == EVENT_ID_HEX_LEN && all_hex
}

pub open spec fn signature_hex_shape_valid(hex_len: int, all_hex: bool) -> bool {
    hex_len == SIGNATURE_HEX_LEN && all_hex
}

pub open spec fn secret_bytes_admission(byte_len: int, crypto_valid: bool) -> SecretKeyAdmissionSpec {
    if byte_len != SECP256K1_SECRET_BYTES {
        SecretKeyAdmissionSpec::WrongLength
    } else if !crypto_valid {
        SecretKeyAdmissionSpec::CryptoRejected
    } else {
        SecretKeyAdmissionSpec::Accepted
    }
}

pub open spec fn persisted_secret_admission(
    hex_len: int,
    all_hex: bool,
    crypto_valid: bool,
) -> PersistedSecretAdmissionSpec {
    if hex_len != SECRET_KEY_HEX_LEN {
        PersistedSecretAdmissionSpec::WrongLength
    } else if !all_hex {
        PersistedSecretAdmissionSpec::NonHex
    } else if !crypto_valid {
        PersistedSecretAdmissionSpec::CryptoRejected
    } else {
        PersistedSecretAdmissionSpec::Accepted
    }
}

pub fn secret_bytes_len_exec() -> (len: u32)
    ensures len == SECP256K1_SECRET_BYTES
{
    32
}

pub fn public_key_hex_len_exec() -> (len: u32)
    ensures len == PUBLIC_KEY_HEX_LEN
{
    64
}

pub fn persisted_secret_hex_len_exec() -> (len: u32)
    ensures len == SECRET_KEY_HEX_LEN
{
    64
}

pub fn signature_hex_len_exec() -> (len: u32)
    ensures len == SIGNATURE_HEX_LEN
{
    128
}

pub proof fn secret_hex_length_matches_secret_bytes()
    ensures hex_len_for_bytes(SECP256K1_SECRET_BYTES) == SECRET_KEY_HEX_LEN
{
}

pub proof fn public_hex_length_matches_xonly_bytes()
    ensures hex_len_for_bytes(XONLY_PUBLIC_KEY_BYTES) == PUBLIC_KEY_HEX_LEN
{
}

pub proof fn event_id_hex_length_matches_event_id_bytes()
    ensures hex_len_for_bytes(EVENT_ID_BYTES) == EVENT_ID_HEX_LEN
{
}

pub proof fn signature_hex_length_matches_signature_bytes()
    ensures hex_len_for_bytes(SCHNORR_SIGNATURE_BYTES) == SIGNATURE_HEX_LEN
{
}

pub proof fn raw_secret_exactly_32_bytes()
    ensures
        raw_secret_shape_valid(SECP256K1_SECRET_BYTES),
        !raw_secret_shape_valid(SECP256K1_SECRET_BYTES - 1),
        !raw_secret_shape_valid(SECP256K1_SECRET_BYTES + 1),
{
}

pub proof fn public_key_hex_requires_hex_and_64_chars()
    ensures
        public_key_hex_shape_valid(PUBLIC_KEY_HEX_LEN, true),
        !public_key_hex_shape_valid(PUBLIC_KEY_HEX_LEN - 1, true),
        !public_key_hex_shape_valid(PUBLIC_KEY_HEX_LEN, false),
{
}

pub proof fn persisted_secret_requires_hex_and_64_chars()
    ensures
        persisted_secret_shape_valid(SECRET_KEY_HEX_LEN, true),
        !persisted_secret_shape_valid(SECRET_KEY_HEX_LEN - 1, true),
        !persisted_secret_shape_valid(SECRET_KEY_HEX_LEN, false),
{
}

pub proof fn signature_hex_requires_128_chars()
    ensures
        signature_hex_shape_valid(SIGNATURE_HEX_LEN, true),
        !signature_hex_shape_valid(SIGNATURE_HEX_LEN - 1, true),
        !signature_hex_shape_valid(SIGNATURE_HEX_LEN, false),
{
}

pub proof fn raw_secret_admission_rejects_wrong_length_before_crypto()
    ensures
        secret_bytes_admission(SECP256K1_SECRET_BYTES - 1, true)
            == SecretKeyAdmissionSpec::WrongLength,
        secret_bytes_admission(SECP256K1_SECRET_BYTES + 1, true)
            == SecretKeyAdmissionSpec::WrongLength,
        secret_bytes_admission(SECP256K1_SECRET_BYTES - 1, false)
            == SecretKeyAdmissionSpec::WrongLength,
{
}

pub proof fn raw_secret_admission_delegates_crypto_after_shape()
    ensures
        secret_bytes_admission(SECP256K1_SECRET_BYTES, false)
            == SecretKeyAdmissionSpec::CryptoRejected,
        secret_bytes_admission(SECP256K1_SECRET_BYTES, true)
            == SecretKeyAdmissionSpec::Accepted,
{
}

pub proof fn persisted_secret_admission_rejects_wrong_length_before_hex_or_crypto()
    ensures
        persisted_secret_admission(SECRET_KEY_HEX_LEN - 1, true, true)
            == PersistedSecretAdmissionSpec::WrongLength,
        persisted_secret_admission(SECRET_KEY_HEX_LEN + 1, false, true)
            == PersistedSecretAdmissionSpec::WrongLength,
        persisted_secret_admission(SECRET_KEY_HEX_LEN - 1, false, false)
            == PersistedSecretAdmissionSpec::WrongLength,
{
}

pub proof fn persisted_secret_admission_rejects_non_hex_before_crypto()
    ensures
        persisted_secret_admission(SECRET_KEY_HEX_LEN, false, true)
            == PersistedSecretAdmissionSpec::NonHex,
        persisted_secret_admission(SECRET_KEY_HEX_LEN, false, false)
            == PersistedSecretAdmissionSpec::NonHex,
{
}

pub proof fn persisted_secret_admission_delegates_crypto_after_shape()
    ensures
        persisted_secret_admission(SECRET_KEY_HEX_LEN, true, false)
            == PersistedSecretAdmissionSpec::CryptoRejected,
        persisted_secret_admission(SECRET_KEY_HEX_LEN, true, true)
            == PersistedSecretAdmissionSpec::Accepted,
{
}

pub proof fn key_material_shapes_are_distinct_where_expected()
    ensures
        SECP256K1_SECRET_BYTES == XONLY_PUBLIC_KEY_BYTES,
        SECRET_KEY_HEX_LEN == PUBLIC_KEY_HEX_LEN,
        SIGNATURE_HEX_LEN > PUBLIC_KEY_HEX_LEN,
        SCHNORR_SIGNATURE_BYTES > XONLY_PUBLIC_KEY_BYTES,
{
}

} // verus!
