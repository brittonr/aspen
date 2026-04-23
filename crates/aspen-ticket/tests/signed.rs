#![cfg(feature = "signed")]

use aspen_ticket::AspenClusterTicket;
use aspen_ticket::ClusterTicketError;
use aspen_ticket::ClusterTopicId;
use aspen_ticket::SignedAspenClusterTicket;
use iroh_base::PublicKey;
use iroh_base::SecretKey;
use serde::Serialize;

const SIGNED_TEST_VERSION: u8 = 1;
const SIGNED_TEST_SECRET_BYTES: [u8; 32] = [21; 32];
const SIGNED_TEST_TOPIC_BYTES: [u8; 32] = [22; 32];
const SIGNED_TEST_NONCE: [u8; 16] = [23; 16];
const SIGNED_TEST_ISSUED_AT_SECS: u64 = 1_000;
const SIGNED_TEST_EXPIRES_AT_SECS: u64 = 2_000;
const SIGNED_TEST_VERIFY_AT_SECS: u64 = 1_500;
const SIGNED_TEST_EXPIRED_AT_SECS: u64 = 2_001;
const SIGNED_TEST_CLUSTER_ID: &str = "signed-only";
const CORRUPTED_TICKET_SUFFIX: &str = "!";

#[derive(Serialize)]
struct SignedPayload<'a> {
    version: u8,
    ticket: &'a AspenClusterTicket,
    issuer: PublicKey,
    issued_at_secs: u64,
    expires_at_secs: u64,
    nonce: [u8; 16],
}

fn build_signed_ticket() -> SignedAspenClusterTicket {
    let secret_key = SecretKey::from(SIGNED_TEST_SECRET_BYTES);
    let ticket = AspenClusterTicket::new(
        ClusterTopicId::from_bytes(SIGNED_TEST_TOPIC_BYTES),
        SIGNED_TEST_CLUSTER_ID.to_string(),
    );
    let issuer = secret_key.public();
    let payload = SignedPayload {
        version: SIGNED_TEST_VERSION,
        ticket: &ticket,
        issuer,
        issued_at_secs: SIGNED_TEST_ISSUED_AT_SECS,
        expires_at_secs: SIGNED_TEST_EXPIRES_AT_SECS,
        nonce: SIGNED_TEST_NONCE,
    };
    let payload_bytes = postcard::to_allocvec(&payload)
        .expect("signed payload serialization should succeed for bounded fields");
    let signature = secret_key.sign(&payload_bytes);
    SignedAspenClusterTicket {
        version: SIGNED_TEST_VERSION,
        ticket,
        issuer,
        issued_at_secs: SIGNED_TEST_ISSUED_AT_SECS,
        expires_at_secs: SIGNED_TEST_EXPIRES_AT_SECS,
        nonce: SIGNED_TEST_NONCE,
        signature,
    }
}

#[test]
fn signed_only_helpers_verify_at_explicit_time() {
    let signed = build_signed_ticket();
    assert!(
        signed.verify_at(SIGNED_TEST_VERIFY_AT_SECS).is_some(),
        "signed-only verify_at should accept a valid ticket"
    );
    assert!(
        signed.verify_with_error_at(SIGNED_TEST_VERIFY_AT_SECS).is_ok(),
        "signed-only verify_with_error_at should accept a valid ticket"
    );
    assert!(
        !signed.is_expired_at(SIGNED_TEST_VERIFY_AT_SECS),
        "signed-only is_expired_at should report false before expiry"
    );
}

#[test]
fn signed_only_helpers_reject_expired_ticket() {
    let signed = build_signed_ticket();
    let error = signed
        .verify_with_error_at(SIGNED_TEST_EXPIRED_AT_SECS)
        .expect_err("expired signed ticket should be rejected");
    match error {
        ClusterTicketError::ExpiredSignedTicket {
            expires_at_secs,
            now_secs,
        } => {
            assert_eq!(expires_at_secs, SIGNED_TEST_EXPIRES_AT_SECS);
            assert_eq!(now_secs, SIGNED_TEST_EXPIRED_AT_SECS);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(
        signed.is_expired_at(SIGNED_TEST_EXPIRED_AT_SECS),
        "signed-only is_expired_at should report true after expiry"
    );
}

#[test]
fn signed_only_deserialize_rejects_corrupted_input() {
    let signed = build_signed_ticket();
    let corrupted = format!("{}{}", signed.serialize(), CORRUPTED_TICKET_SUFFIX);
    let error = SignedAspenClusterTicket::deserialize(&corrupted)
        .expect_err("corrupted signed ticket should fail to deserialize");
    match error {
        ClusterTicketError::Deserialize { reason } => {
            assert!(!reason.is_empty(), "deserialize reason should not be empty");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
