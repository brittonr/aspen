use super::*;

const MAX_EFFECT_OUTPUT_BYTES: usize = 4_096;
const OVERBOUND_EFFECT_OUTPUT_BYTES: usize = MAX_EFFECT_OUTPUT_BYTES + 1;

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

// r[verify molten.system_extension.native_host.effect_completion_value.accepted]
// r[verify molten.system_extension.native_host.effect_completion_value.rejected]
// r[verify molten.system_extension.native_host.effect_completion_value.compatibility]
#[test]
fn materialized_effect_output_requires_exact_bounded_bytes_for_native_profiles() {
    let profile = admit_native_host_profile(&profile()).expect("native host profile");
    assert_eq!(u64::try_from(MAX_EFFECT_OUTPUT_BYTES), Ok(MAX_CALLBACK_BYTES),);
    let bytes = b"provider-terminal-observation".to_vec();
    let output_ref = format!("blake3:{}", blake3::hash(&bytes).to_hex());
    let output = NativeCallbackValue {
        value_ref: output_ref.clone(),
        bytes,
    };
    admit_native_effect_output(&profile, &output_ref, Some(&output)).expect("exact materialized effect output");

    let mut generic_profile = profile.clone();
    generic_profile.profile.requires_materialized_values = false;
    admit_native_effect_output(&generic_profile, &output_ref, None).expect("generic reference-only effect output");

    let missing = admit_native_effect_output(&profile, &output_ref, None).expect_err("reference-only output must deny");
    assert!(missing.iter().any(|issue| matches!(issue, NativeHostIssue::MaterializedValuesRequired)));

    let mut mismatched = output.clone();
    mismatched.bytes.push(0);
    let mismatch =
        admit_native_effect_output(&profile, &output_ref, Some(&mismatched)).expect_err("identity mismatch must deny");
    assert!(
        mismatch
            .iter()
            .any(|issue| matches!(issue, NativeHostIssue::ValueIdentityMismatch("effect-output")))
    );

    let oversized_bytes = vec![0; OVERBOUND_EFFECT_OUTPUT_BYTES];
    let oversized_ref = format!("blake3:{}", blake3::hash(&oversized_bytes).to_hex());
    let oversized = NativeCallbackValue {
        value_ref: oversized_ref.clone(),
        bytes: oversized_bytes,
    };
    let overbound =
        admit_native_effect_output(&profile, &oversized_ref, Some(&oversized)).expect_err("overbound output must deny");
    assert!(overbound.iter().any(|issue| matches!(issue, NativeHostIssue::CallbackBytesExceeded { .. })));
}
