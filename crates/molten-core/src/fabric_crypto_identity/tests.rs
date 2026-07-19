use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::*;

const SYNTHETIC_REF_CHUNK_HEX_CHARS: usize = 16;
const SYNTHETIC_REF_CHUNK_REPETITIONS: usize = 4;
const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = 2;
const ED25519_SIGNATURE_BYTES: u64 = 64;

fn test_ref(label: &str) -> String {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let digest = hasher.finish();
    let chunk = format!("{digest:0width$x}", width = SYNTHETIC_REF_CHUNK_HEX_CHARS);
    format!("blake3:{}", chunk.repeat(SYNTHETIC_REF_CHUNK_REPETITIONS))
}

fn production_profile() -> CryptoAdapterProfile {
    CryptoAdapterProfile {
        schema: CRYPTO_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "ed25519-iroh-production-v1".to_string(),
        profile_ref: test_ref("production-profile"),
        class: CryptoProfileClass::Production,
        algorithm: CryptoAlgorithm::Ed25519Iroh,
        backend_classes: vec![KeyBackendClass::CapabilityFile, KeyBackendClass::ManagedSecret],
        allowed_purposes: vec![
            KeyPurpose::TransportEndpoint,
            KeyPurpose::FederationOrigin,
            KeyPurpose::Delegation,
            KeyPurpose::EvidenceSigning,
            KeyPurpose::Authority,
        ],
        entropy_profile_ref: Some(test_ref("os-csprng-entropy")),
        domain_version: "v1".to_string(),
        allow_key_sharing: false,
        sharing_policy_ref: None,
        max_signature_bytes: MAX_SIGNATURE_BYTES,
        non_claims: REQUIRED_CRYPTO_NON_CLAIMS.to_vec(),
    }
}

fn fixture_profile() -> CryptoAdapterProfile {
    CryptoAdapterProfile {
        schema: CRYPTO_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "blake3-fixture-v1".to_string(),
        profile_ref: test_ref("fixture-profile"),
        class: CryptoProfileClass::FixtureSimulation,
        algorithm: CryptoAlgorithm::Blake3Fixture,
        backend_classes: vec![KeyBackendClass::InMemoryFixture],
        allowed_purposes: vec![KeyPurpose::FederationOrigin],
        entropy_profile_ref: None,
        domain_version: "fixture-v1".to_string(),
        allow_key_sharing: false,
        sharing_policy_ref: None,
        max_signature_bytes: MAX_SIGNATURE_BYTES,
        non_claims: REQUIRED_CRYPTO_NON_CLAIMS.to_vec(),
    }
}

fn handle(purpose: KeyPurpose, generation: u64, currentness: KeyCurrentness) -> OpaqueKeyHandle {
    OpaqueKeyHandle {
        schema: OPAQUE_KEY_HANDLE_SCHEMA.to_string(),
        handle_ref: test_ref(&format!("handle-{}-{generation}", purpose.as_str())),
        profile_ref: test_ref("production-profile"),
        purpose,
        generation,
        public_key_ref: test_ref(&format!("public-{}-{generation}", purpose.as_str())),
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: test_ref("capability-file-backend"),
        currentness,
        currentness_evidence_ref: test_ref(&format!("currentness-{generation}")),
        secret_material_exposed: false,
    }
}

fn domain(purpose: KeyPurpose, public_key_ref: &str) -> SignatureDomain {
    SignatureDomain {
        schema: SIGNATURE_DOMAIN_SCHEMA.to_string(),
        domain_id: format!("{}-domain", purpose.as_str()),
        domain_version: "v1".to_string(),
        purpose,
        payload_schema: "canonical-payload-v1".to_string(),
        payload_ref: test_ref("canonical-payload"),
        signer_public_ref: public_key_ref.to_string(),
        verifier_context_ref: test_ref("verifier-context"),
    }
}

fn sign_request(handle: &OpaqueKeyHandle) -> SignRequest {
    SignRequest {
        operation_id: "sign-operation".to_string(),
        profile_ref: handle.profile_ref.clone(),
        handle: handle.clone(),
        domain: domain(handle.purpose, &handle.public_key_ref),
        current_generation: handle.generation,
        current_handle_ref: handle.handle_ref.clone(),
        policy_ref: test_ref("sign-policy"),
    }
}

fn verification_request(profile: &CryptoAdapterProfile, handle: &OpaqueKeyHandle) -> VerificationRequest {
    let expected_domain = domain(handle.purpose, &handle.public_key_ref);
    VerificationRequest {
        operation_id: "verify-operation".to_string(),
        profile_ref: profile.profile_ref.clone(),
        expected_domain: expected_domain.clone(),
        observed: SignatureMetadata {
            profile_ref: profile.profile_ref.clone(),
            algorithm: profile.algorithm,
            purpose: handle.purpose,
            generation: handle.generation,
            signer_public_ref: handle.public_key_ref.clone(),
            domain_ref: test_ref("domain-value"),
            payload_ref: expected_domain.payload_ref,
            verifier_context_ref: expected_domain.verifier_context_ref,
            signature_ref: test_ref("signature"),
            signature_bytes: ED25519_SIGNATURE_BYTES,
        },
        cryptographic_verification_passed: true,
        signer_currentness: handle.currentness,
        signer_generation: handle.generation,
        policy_ref: test_ref("verify-policy"),
    }
}

// r[verify molten.crypto_identity.adapter_contract]
// r[verify molten.crypto_identity.fixture_profile_boundary]
#[test]
fn production_profile_requires_ed25519_entropy_and_production_backend() {
    let profile = production_profile();
    assert!(validate_crypto_profile(&profile).is_empty());
    admit_profile_for_production(&profile).expect("production profile admitted");

    let fixture = fixture_profile();
    let denied = admit_profile_for_production(&fixture).expect_err("fixture profile denied in production");
    assert!(denied.contains(&CryptoIdentityIssue::FixtureProfileDeniedInProduction));
    assert!(denied.contains(&CryptoIdentityIssue::UnsupportedProductionAlgorithm));

    let mut unsafe_profile = profile.clone();
    unsafe_profile.backend_classes = vec![KeyBackendClass::InMemoryFixture];
    unsafe_profile.entropy_profile_ref = None;
    let issues = validate_crypto_profile(&unsafe_profile);
    assert!(issues.contains(&CryptoIdentityIssue::FixtureBackendDeniedInProduction));
    assert!(issues.contains(&CryptoIdentityIssue::MissingEntropyProfile));
}

// r[verify molten.crypto_identity.adapter_contract]
#[test]
fn key_generation_is_purpose_backend_entropy_and_policy_bound() {
    let profile = production_profile();
    let request = KeyGenerationRequest {
        operation_id: "first-boot-generation".to_string(),
        profile_ref: profile.profile_ref.clone(),
        purpose: KeyPurpose::TransportEndpoint,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: test_ref("capability-file-backend"),
        entropy_profile_ref: profile.entropy_profile_ref.clone().expect("entropy"),
        generation: GENERATION_ONE,
        policy_ref: test_ref("generation-policy"),
        permit_first_boot_generation: true,
    };
    let plan = admit_key_generation(&profile, &request).expect("generation plan");
    assert!(plan.persist_restricted);
    assert!(!plan.replace_existing);

    let mut denied = request;
    denied.backend_class = KeyBackendClass::InMemoryFixture;
    let issues = admit_key_generation(&profile, &denied).expect_err("fixture backend denied");
    assert!(issues.contains(&CryptoIdentityIssue::UnsupportedBackend(KeyBackendClass::InMemoryFixture)));
}

// r[verify molten.crypto_identity.purpose_domain_separation]
// r[verify molten.crypto_identity.canonical_signature_binding]
#[test]
fn signing_binds_purpose_domain_public_key_payload_and_current_handle() {
    let profile = production_profile();
    let handle = handle(KeyPurpose::FederationOrigin, GENERATION_ONE, KeyCurrentness::Current);
    let request = sign_request(&handle);
    let plan = plan_sign(&profile, &request).expect("sign plan");
    assert_eq!(plan.domain.payload_ref, test_ref("canonical-payload"));
    assert_eq!(plan.purpose, KeyPurpose::FederationOrigin);

    let mut wrong_purpose = request.clone();
    wrong_purpose.domain.purpose = KeyPurpose::Delegation;
    let issues = plan_sign(&profile, &wrong_purpose).expect_err("cross-purpose signing denied");
    assert!(issues.contains(&CryptoIdentityIssue::PurposeMismatch));

    let mut stale = request.clone();
    stale.current_generation = GENERATION_TWO;
    stale.current_handle_ref = test_ref("replacement-handle");
    let issues = plan_sign(&profile, &stale).expect_err("stale handle denied");
    assert!(issues.iter().any(|issue| matches!(issue, CryptoIdentityIssue::HandleGenerationStale { .. })));
    assert!(issues.contains(&CryptoIdentityIssue::HandleRefStale));

    let mut leaked = request;
    leaked.handle.secret_material_exposed = true;
    let issues = plan_sign(&profile, &leaked).expect_err("secret-bearing handle denied");
    assert!(issues.contains(&CryptoIdentityIssue::SecretMaterialExposed));
}

// r[verify molten.crypto_identity.canonical_signature_binding]
#[test]
fn verification_consumes_supplied_crypto_outcome_without_promoting_signature_to_authority() {
    let profile = production_profile();
    let handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Current);
    let expected_domain = domain(handle.purpose, &handle.public_key_ref);
    let metadata = SignatureMetadata {
        profile_ref: profile.profile_ref.clone(),
        algorithm: profile.algorithm,
        purpose: handle.purpose,
        generation: handle.generation,
        signer_public_ref: handle.public_key_ref.clone(),
        domain_ref: test_ref("domain-value"),
        payload_ref: expected_domain.payload_ref.clone(),
        verifier_context_ref: expected_domain.verifier_context_ref.clone(),
        signature_ref: test_ref("signature"),
        signature_bytes: ED25519_SIGNATURE_BYTES,
    };
    let request = VerificationRequest {
        operation_id: "verify-operation".to_string(),
        profile_ref: profile.profile_ref.clone(),
        expected_domain: expected_domain.clone(),
        observed: metadata.clone(),
        cryptographic_verification_passed: true,
        signer_currentness: KeyCurrentness::Current,
        signer_generation: GENERATION_ONE,
        policy_ref: test_ref("verify-policy"),
    };
    let accepted = evaluate_verification(&profile, &request);
    assert_eq!(accepted.kind, VerificationDecisionKind::Accept);
    assert!(accepted.issues.is_empty());

    let mut tampered = request.clone();
    tampered.observed.payload_ref = test_ref("tampered-payload");
    tampered.cryptographic_verification_passed = false;
    let denied = evaluate_verification(&profile, &tampered);
    assert_eq!(denied.kind, VerificationDecisionKind::Deny);
    assert!(denied.issues.contains(&CryptoIdentityIssue::PayloadRefMismatch));
    assert!(denied.issues.contains(&CryptoIdentityIssue::CryptographicVerificationFailed));

    let mut revoked = request;
    revoked.signer_currentness = KeyCurrentness::Revoked;
    let denied = evaluate_verification(&profile, &revoked);
    assert!(denied.issues.contains(&CryptoIdentityIssue::HandleNotCurrent(KeyCurrentness::Revoked)));
}

// r[verify molten.crypto_identity.rotation_revocation]
#[test]
fn rotation_advances_generation_and_requires_explicit_overlap_or_revocation() {
    let profile = production_profile();
    let current = handle(KeyPurpose::TransportEndpoint, GENERATION_ONE, KeyCurrentness::Current);
    let request = KeyRotationRequest {
        operation_id: "rotate-transport".to_string(),
        profile_ref: profile.profile_ref.clone(),
        purpose: current.purpose,
        backend_class: current.backend_class,
        backend_ref: current.backend_ref.clone(),
        old_handle_ref: current.handle_ref.clone(),
        old_public_key_ref: current.public_key_ref.clone(),
        old_generation: GENERATION_ONE,
        new_generation: GENERATION_TWO,
        policy_ref: test_ref("rotation-policy"),
        activation_boundary_ref: test_ref("rotation-activation"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(test_ref("rotation-revocation")),
    };
    let plan = plan_key_rotation(&profile, &current, &request).expect("rotation plan");
    assert_eq!(plan.old_key_next_currentness, KeyCurrentness::Revoked);
    let next = handle(KeyPurpose::TransportEndpoint, GENERATION_TWO, KeyCurrentness::Current);
    let outcome = complete_key_rotation(&plan, &next).expect("rotation complete");
    assert_eq!(outcome.new_generation, GENERATION_TWO);
    assert_eq!(outcome.old_key_currentness, KeyCurrentness::Revoked);

    let mut no_revocation = request.clone();
    no_revocation.revocation_evidence_ref = None;
    let issues = plan_key_rotation(&profile, &current, &no_revocation).expect_err("missing revocation denied");
    assert!(issues.contains(&CryptoIdentityIssue::RevocationEvidenceRequired));

    let mut stale = request;
    stale.new_generation = GENERATION_ONE;
    let issues = plan_key_rotation(&profile, &current, &stale).expect_err("non-advancing rotation denied");
    assert!(issues.contains(&CryptoIdentityIssue::GenerationNotAdvanced));
}

// r[verify molten.crypto_identity.redaction]
#[test]
fn redaction_keeps_public_status_and_denies_private_material() {
    let profile = production_profile();
    let input = AdapterDiagnosticInput {
        profile_ref: profile.profile_ref,
        purpose: KeyPurpose::Authority,
        generation: Some(GENERATION_ONE),
        currentness: Some(KeyCurrentness::Current),
        permission_status: AdapterPermissionStatus::Restricted,
        backend_class: KeyBackendClass::ManagedSecret,
        public_key_ref: Some(test_ref("authority-public")),
        receipt_refs: vec![test_ref("status-receipt")],
        backend_locator: Some("vault://sensitive/path".to_string()),
        raw_error: Some("backend token leaked in raw error".to_string()),
        bearer_token: Some("secret-token".to_string()),
        private_material_present: false,
    };
    let redacted = redact_adapter_status(&input).expect("redacted status");
    assert_eq!(redacted.currentness, Some(KeyCurrentness::Current));
    assert_eq!(redacted.permission_status, AdapterPermissionStatus::Restricted);
    assert!(redacted.has_redacted_backend_locator);
    assert!(redacted.has_redacted_error);
    assert!(redacted.has_redacted_bearer_token);

    let mut leaking = input;
    leaking.private_material_present = true;
    let issues = redact_adapter_status(&leaking).expect_err("private material denied");
    assert!(issues.contains(&CryptoIdentityIssue::DiagnosticSecretLeak));
}

// r[verify molten.artifact_auth_adoption.authority]
// r[verify molten.artifact_auth_adoption.cutover]
#[test]
fn artifact_auth_dual_run_maps_current_identity_without_promoting_authority() {
    let profile = production_profile();
    let handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Current);
    let request = verification_request(&profile, &handle);
    let standalone = standalone_observation(&handle.public_key_ref, true).expect("standalone observation");
    let report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &request,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &handle.currentness_evidence_ref,
        standalone_cryptographic: standalone,
    });

    assert_eq!(report.legacy.kind, VerificationDecisionKind::Accept);
    assert!(report.standalone.as_ref().is_some_and(|decision| decision.passed));
    assert!(report.compatibility.case_explained);
    assert!(!report.compatibility.decision_drift);
    assert!(!report.compatibility.non_claim_drift);
    assert!(report.compatibility.legacy_authoritative);
    assert!(!report.compatibility.standalone_authority_admitted);
    assert!(report.compatibility.rollback_available);
    assert!(report.opaque_handle_authority_retained);
    assert!(report.backend_authority_retained);
    assert!(report.rotation_authority_retained);
}

// r[verify molten.artifact_auth_adoption.cutover]
#[test]
fn artifact_auth_dual_run_classifies_tamper_revocation_and_false_parity() {
    let profile = production_profile();
    let current_handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Current);
    let mut tampered = verification_request(&profile, &current_handle);
    tampered.observed.payload_ref = test_ref("tampered-payload");
    tampered.cryptographic_verification_passed = false;
    let rejected = standalone_observation(&current_handle.public_key_ref, false).expect("rejected observation");
    let report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &tampered,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: rejected,
    });
    assert!(report.compatibility.case_explained);
    assert!(report.compatibility.mapped_failure_causes.contains(&"payload".to_string()));
    assert!(report.compatibility.mapped_failure_causes.contains(&"signature".to_string()));

    let standalone_pass = standalone_observation(&current_handle.public_key_ref, true).expect("passing observation");
    let false_parity = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &tampered,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_pass,
    });
    assert!(false_parity.compatibility.decision_drift);
    assert!(!false_parity.compatibility.case_explained);

    let revoked_handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Revoked);
    let revoked_request = verification_request(&profile, &revoked_handle);
    let revoked_crypto = standalone_observation(&revoked_handle.public_key_ref, true).expect("revoked observation");
    let revoked = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &revoked_request,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &revoked_handle.currentness_evidence_ref,
        standalone_cryptographic: revoked_crypto,
    });
    assert!(revoked.compatibility.case_explained);
    assert!(revoked.compatibility.mapped_failure_causes.contains(&"currentness".to_string()));
}

// r[verify molten.artifact_auth_adoption.authority]
#[test]
fn artifact_auth_rejects_lossy_key_mapping_and_keeps_overlap_verification_bounded() {
    let profile = production_profile();
    let mut malformed = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Current);
    malformed.public_key_ref = "label-only-key".to_string();
    let malformed_request = verification_request(&profile, &malformed);
    let malformed_crypto = artifact_auth_core::CryptographicObservation {
        algorithm: artifact_auth_core::ALGORITHM_ED25519.to_string(),
        key_identity: artifact_auth_core::ArtifactRef {
            profile: artifact_auth_core::ED25519_PUBLIC_KEY_PROFILE_V1.to_string(),
            algorithm: artifact_auth_core::ALGORITHM_BLAKE3.to_string(),
            digest_hex: "0".repeat(artifact_auth_core::DIGEST_HEX_CHARS),
        },
        verified: true,
        failure_code: None,
    };
    let malformed_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &malformed_request,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &malformed.currentness_evidence_ref,
        standalone_cryptographic: malformed_crypto,
    });
    assert!(malformed_report.standalone.is_none());
    assert!(!malformed_report.compatibility.case_explained);

    let overlap_handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Overlap);
    let overlap_request = verification_request(&profile, &overlap_handle);
    let overlap_crypto = standalone_observation(&overlap_handle.public_key_ref, true).expect("overlap observation");
    let overlap = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &overlap_request,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &overlap_handle.currentness_evidence_ref,
        standalone_cryptographic: overlap_crypto,
    });
    assert!(overlap.compatibility.case_explained);
    assert!(!overlap.compatibility.standalone_authority_admitted);
    assert!(overlap.authority_boundary.contains("signing"));
}

// r[verify molten.artifact_auth_adoption.authority]
// r[verify molten.artifact_auth_adoption.cutover]
#[test]
fn artifact_auth_blocks_profile_context_generation_and_key_drift() {
    let profile = production_profile();
    let current_handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Current);

    let mut wrong_profile = verification_request(&profile, &current_handle);
    wrong_profile.profile_ref = test_ref("wrong-profile");
    let wrong_profile_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &wrong_profile,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_observation(&current_handle.public_key_ref, true)
            .expect("profile observation"),
    });
    assert!(wrong_profile_report.compatibility.decision_drift);
    assert!(!wrong_profile_report.compatibility.case_explained);

    let mut wrong_context = verification_request(&profile, &current_handle);
    wrong_context.observed.verifier_context_ref = test_ref("wrong-verifier-context");
    wrong_context.cryptographic_verification_passed = false;
    let wrong_context_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &wrong_context,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_observation(&current_handle.public_key_ref, false)
            .expect("context observation"),
    });
    assert!(wrong_context_report.compatibility.case_explained);
    assert!(wrong_context_report.compatibility.mapped_failure_causes.contains(&"verifier-context".to_string()));

    let mut stale_generation = verification_request(&profile, &current_handle);
    stale_generation.observed.generation = GENERATION_TWO;
    stale_generation.cryptographic_verification_passed = false;
    let stale_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &stale_generation,
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_observation(&current_handle.public_key_ref, false)
            .expect("stale observation"),
    });
    assert!(stale_report.compatibility.case_explained);
    assert!(stale_report.compatibility.mapped_failure_causes.contains(&"generation".to_string()));

    let wrong_key_ref = test_ref("wrong-standalone-key");
    let wrong_key_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &verification_request(&profile, &current_handle),
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &current_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_observation(&wrong_key_ref, true).expect("wrong-key observation"),
    });
    assert!(!wrong_key_report.compatibility.case_explained);
    assert!(wrong_key_report.compatibility.blockers.contains(&"identity-drift".to_string()));

    let superseded_handle = handle(KeyPurpose::EvidenceSigning, GENERATION_ONE, KeyCurrentness::Superseded);
    let superseded_report = evaluate_artifact_auth_dual_run(&MoltenArtifactAuthObservation {
        profile: &profile,
        request: &verification_request(&profile, &superseded_handle),
        producer_id: "molten-evidence-producer",
        key_id: "evidence-signing-key",
        currentness_ref: &superseded_handle.currentness_evidence_ref,
        standalone_cryptographic: standalone_observation(&superseded_handle.public_key_ref, true)
            .expect("superseded observation"),
    });
    assert!(superseded_report.compatibility.case_explained);
    assert!(superseded_report.compatibility.mapped_failure_causes.contains(&"currentness".to_string()));
}
