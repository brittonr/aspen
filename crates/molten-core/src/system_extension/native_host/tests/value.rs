use super::*;

// r[verify molten.system_extension.native_host.profile]
// r[verify molten.system_extension.native_host.nonclaims]
// r[verify molten.system_extension.native_host.value_protocol]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn exact_local_pilot_profile_passes_and_missing_nonclaim_denies() {
    let admitted = admit_native_host_profile(&profile()).expect("native host profile");
    assert_eq!(admitted.profile.alpn, NATIVE_ALPN);
    assert_eq!(admitted.profile.non_claims, REQUIRED_NATIVE_HOST_NON_CLAIMS);

    let mut invalid = profile();
    invalid.non_claims.pop();
    let issues = admit_native_host_profile(&invalid).expect_err("missing non-claim must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::MissingNonClaim(_))));

    let mut reference_only = profile();
    reference_only.requires_materialized_values = false;
    let issues = admit_native_host_profile(&reference_only).expect_err("reference-only profile must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::MaterializedValuesRequired)));
}

// r[verify molten.system_extension.native_host.ingress]
// r[verify molten.system_extension.native_host.value_materialization]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn ingress_binds_transport_values_and_service_acceptance_separately() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    let admitted = admit_native_ingress(&profile, &instance(), &ingress()).expect("ingress admission");
    assert!(admitted.acknowledgement_ref.starts_with("blake3:"));

    let mut stale = ingress();
    stale.generation = STALE_GENERATION;
    let issues = admit_native_ingress(&profile, &instance(), &stale).expect_err("stale ingress must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::StaleGeneration { .. })));

    let mut corrupt = ingress();
    corrupt.payload.bytes.push(0);
    let issues = admit_native_ingress(&profile, &instance(), &corrupt).expect_err("corrupt ingress must deny");
    assert!(issues.iter().any(|issue| matches!(issue, NativeHostIssue::ValueIdentityMismatch("payload"))));
}
