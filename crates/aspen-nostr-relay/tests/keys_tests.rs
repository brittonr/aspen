//! Tests for Nostr key management — generation, persistence, signing.

use aspen_nostr_relay::NostrIdentity;
use nostr::prelude::*;

#[test]
fn test_generate_produces_valid_keypair() {
    let id = NostrIdentity::generate();
    let hex = id.public_key_hex();
    assert_eq!(hex.len(), 64, "x-only pubkey should be 64 hex chars");
    // Verify hex is valid
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_roundtrip_secret_bytes() {
    let id = NostrIdentity::generate();
    let bytes = id.secret_bytes();
    let restored = NostrIdentity::from_secret_bytes(&bytes).unwrap();
    assert_eq!(id.public_key_hex(), restored.public_key_hex());
}

#[test]
fn test_roundtrip_persist() {
    let id = NostrIdentity::generate();
    let persist = id.to_persist();

    // Verify the persist form is a hex string
    assert_eq!(persist.secret_key_hex.len(), 64);

    let restored = NostrIdentity::from_persist(&persist).unwrap();
    assert_eq!(id.public_key_hex(), restored.public_key_hex());
}

#[test]
fn test_sign_event_produces_valid_signature() {
    let id = NostrIdentity::generate();

    let event = id.sign_event(EventBuilder::text_note("hello from aspen")).unwrap();

    // Verify the event
    assert!(event.verify().is_ok(), "signed event should verify");
    assert_eq!(event.pubkey, id.keys().public_key());
    assert_eq!(event.content, "hello from aspen");
}

#[test]
fn test_signed_event_verifiable_by_external() {
    let id = NostrIdentity::generate();
    let event = id.sign_event(EventBuilder::text_note("cross-verify")).unwrap();

    // Serialize → deserialize → verify (simulates external client)
    let json = event.as_json();
    let parsed = Event::from_json(&json).unwrap();
    assert!(parsed.verify().is_ok());
    assert_eq!(parsed.id, event.id);
}

#[test]
fn test_two_identities_are_independent() {
    let id1 = NostrIdentity::generate();
    let id2 = NostrIdentity::generate();
    assert_ne!(id1.public_key_hex(), id2.public_key_hex());
    assert_ne!(id1.secret_bytes(), id2.secret_bytes());
}

#[test]
fn test_from_invalid_secret_bytes() {
    // All zeros is not a valid secp256k1 secret key
    let result = NostrIdentity::from_secret_bytes(&[0u8; 32]);
    assert!(result.is_err());
}
