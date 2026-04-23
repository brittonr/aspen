#![cfg(feature = "std")]

use aspen_ticket::{AspenClusterTicket, ClusterTopicId, SignedAspenClusterTicket};

#[test]
fn std_signed_wrappers_work() {
    let secret_key = iroh_base::SecretKey::from([5u8; 32]);
    let ticket = AspenClusterTicket::new(ClusterTopicId::from_bytes([11u8; 32]), "std-cluster".to_string());

    let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).expect("std sign should succeed");
    assert!(signed.verify().is_some(), "std verify should succeed");
    assert!(!signed.is_expired(), "fresh signed ticket should not be expired");
}

#[test]
fn std_sign_with_validity_uses_current_time_wrappers() {
    let secret_key = iroh_base::SecretKey::from([6u8; 32]);
    let ticket = AspenClusterTicket::new(ClusterTopicId::from_bytes([12u8; 32]), "std-validity".to_string());

    let signed = SignedAspenClusterTicket::sign_with_validity(ticket, &secret_key, 60).expect("std sign_with_validity should succeed");
    assert!(signed.verify().is_some(), "std verify should succeed after sign_with_validity");
}
