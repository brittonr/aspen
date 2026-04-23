#![cfg(feature = "std")]

use aspen_ticket::{AspenClusterTicket, ClusterTopicId, SignedAspenClusterTicket};

const STD_SIGN_SECRET_BYTES: [u8; 32] = [5; 32];
const STD_VALIDITY_SECRET_BYTES: [u8; 32] = [6; 32];
const STD_SIGN_TOPIC_BYTES: [u8; 32] = [11; 32];
const STD_VALIDITY_TOPIC_BYTES: [u8; 32] = [12; 32];
const STD_VALIDITY_SECS: u64 = 60;
const STD_SIGN_CLUSTER_ID: &str = "std-cluster";
const STD_VALIDITY_CLUSTER_ID: &str = "std-validity";

#[test]
fn std_signed_wrappers_work() {
    let secret_key = iroh_base::SecretKey::from(STD_SIGN_SECRET_BYTES);
    let ticket = AspenClusterTicket::new(
        ClusterTopicId::from_bytes(STD_SIGN_TOPIC_BYTES),
        STD_SIGN_CLUSTER_ID.to_string(),
    );

    let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).expect("std sign should succeed");
    assert!(signed.verify().is_some(), "std verify should succeed");
    assert!(!signed.is_expired(), "fresh signed ticket should not be expired");
    assert!(
        signed.verify_at(signed.issued_at_secs).is_some(),
        "std surface should still expose signed-only explicit-time helpers"
    );
}

#[test]
fn std_sign_with_validity_uses_current_time_wrappers() {
    let secret_key = iroh_base::SecretKey::from(STD_VALIDITY_SECRET_BYTES);
    let ticket = AspenClusterTicket::new(
        ClusterTopicId::from_bytes(STD_VALIDITY_TOPIC_BYTES),
        STD_VALIDITY_CLUSTER_ID.to_string(),
    );

    let signed = SignedAspenClusterTicket::sign_with_validity(ticket, &secret_key, STD_VALIDITY_SECS)
        .expect("std sign_with_validity should succeed");
    assert!(signed.verify().is_some(), "std verify should succeed after sign_with_validity");
    assert!(
        signed.verify_at(signed.issued_at_secs).is_some(),
        "std surface should keep explicit-time verification available"
    );
}
