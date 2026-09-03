use molten_core::coordination_delivery::*;
use serde::Deserialize;

use super::support::*;

const GENERATED_PROFILE: &str = "config/coordination-delivery/generated/profile.json";

#[derive(Debug, Deserialize)]
struct ProfileProjection {
    schema: String,
    policy: DeliveryPolicy,
    manifest: DeliveryManifest,
    time_domain: String,
    jitter: String,
    base_fifo_unchanged: bool,
    inline_payloads_allowed: bool,
    claims_exactly_once: bool,
    receipt_grants_authority: bool,
    receipt_establishes_release_eligibility: bool,
}

// r[verify molten.coordination_delivery.versioned_extension]
// r[verify molten.coordination_delivery.final_validation]
#[test]
fn nickel_profile_matches_the_rust_policy_and_manifest_projection() {
    let bytes = std::fs::read(GENERATED_PROFILE).expect("generated delivery profile");
    let projection = serde_json::from_slice::<ProfileProjection>(&bytes).expect("parse generated delivery profile");
    assert_eq!(projection.schema, "molten.coordination-delivery-profile.v1");
    assert_eq!(projection.policy, policy());
    assert_eq!(projection.manifest, manifest(&projection.policy));
    assert_eq!(identify_delivery_policy(&projection.policy), projection.manifest.policy_ref);
    assert_eq!(projection.time_domain, "logical");
    assert_eq!(projection.jitter, "none");
    assert!(projection.base_fifo_unchanged);
    assert!(!projection.inline_payloads_allowed);
    assert!(!projection.claims_exactly_once);
    assert!(!projection.receipt_grants_authority);
    assert!(!projection.receipt_establishes_release_eligibility);
}
