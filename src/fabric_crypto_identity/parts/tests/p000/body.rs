use super::*;

const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = 2;
const OWNER_ONLY_SECRET_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const NON_OWNER_PERMISSION_MASK: u32 = 0o077;

fn test_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn profile() -> CanonicalCryptoProfile {
    canonical_crypto_profile(&production_ed25519_profile(test_ref("production-profile"), test_ref("os-csprng-entropy")))
        .expect("production profile")
}

fn adapter<'a>(namespace: &'a crate::node_state::NodeStateNamespace) -> IrohEd25519FileAdapter<'a> {
    IrohEd25519FileAdapter::new(namespace, profile(), test_ref("capability-file-backend")).expect("file adapter")
}

fn domain(
    profile: &CanonicalCryptoProfile,
    purpose: KeyPurpose,
    public_key_ref: &str,
    payload_label: &str,
) -> CanonicalSignatureDomain {
    canonical_signature_domain(profile, &SignatureDomain {
        schema: SIGNATURE_DOMAIN_SCHEMA.to_string(),
        domain_id: format!("{}-domain", purpose.as_str()),
        domain_version: profile.profile.domain_version.clone(),
        purpose,
        payload_schema: "canonical-preserves-payload-v1".to_string(),
        payload_ref: test_ref(payload_label),
        signer_public_ref: public_key_ref.to_string(),
        verifier_context_ref: test_ref("verifier-context"),
    })
    .expect("canonical domain")
}

// r[verify molten.crypto_identity.production_key_lifecycle]
// r[verify molten.crypto_identity.adapter_conformance]
#[test]
fn production_file_key_is_random_persisted_restricted_and_restart_stable() {
    let workspace = temp_dir("crypto-production-restart");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Identity, &workspace)
            .expect("identity namespace");
    let adapter = adapter(&namespace);
    let first = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("first boot generation");
    assert!(first.generated);
    assert_eq!(first.handle.handle.generation, GENERATION_ONE);
    assert_eq!(first.permission_status, KeyPermissionStatus::Restricted);
    let second = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), false)
        .expect("restart resolution");
    assert!(!second.generated);
    assert_eq!(first.handle.handle, second.handle.handle);
    assert_eq!(first.public_key, second.public_key);

    let key_path = transport_key_path().expect("transport key path");
    let mode = namespace.unix_mode(&key_path).expect("mode").expect("unix mode");
    #[cfg(unix)]
    assert_eq!(mode & NON_OWNER_PERMISSION_MASK, 0);
    let secret_record = namespace.read(&key_path, crate::node_state::MAX_NODE_SECRET_BYTES).expect("secret record");
    let status = adapter
        .redacted_status(KeyPurpose::TransportEndpoint, vec![test_ref("status-receipt")])
        .expect("status");
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(!status_text.as_bytes().windows(secret_record.len()).any(|window| window == secret_record));
    assert_eq!(status.status.currentness, Some(KeyCurrentness::Current));
    assert_eq!(status.status.permission_status, AdapterPermissionStatus::Restricted);
    assert!(status.status.has_redacted_backend_locator);
}

// r[verify molten.crypto_identity.canonical_signature_binding]
// r[verify molten.crypto_identity.purpose_domain_separation]
// r[verify molten.crypto_identity.adapter_conformance]
#[test]
fn production_sign_verify_binds_domain_and_denies_wrong_purpose_or_payload() {
    let workspace = temp_dir("crypto-production-sign");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Secrets, &workspace)
            .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::FederationOrigin, &test_ref("generation-policy"), true)
        .expect("federation key");
    let signed_domain =
        domain(adapter.profile(), key.handle.handle.purpose, &key.handle.handle.public_key_ref, "inventory");
    let signature = sign_federation_payload(&adapter, &key.handle.handle, &signed_domain, &test_ref("sign-policy"))
        .expect("production federation signature");
    let verify_policy_ref = test_ref("verify-policy");
    let verify_input = VerificationInput {
        expected_domain: &signed_domain,
        signature: &signature,
        signer_currentness: KeyCurrentness::Current,
        signer_generation: key.handle.handle.generation,
        policy_ref: &verify_policy_ref,
    };
    let verified = adapter.verify(&key.public_key, verify_input).expect("verification outcome");
    assert_eq!(verified.decision.kind, VerificationDecisionKind::Accept);
    admit_federation_verification(&verified).expect("federation verification admission");

    let other_public_key = other_federation_public_key();
    let wrong_key = adapter.verify(&other_public_key, verify_input).expect("wrong key outcome");
    assert_eq!(wrong_key.decision.kind, VerificationDecisionKind::Deny);
    assert!(wrong_key.decision.issues.contains(&CryptoIdentityIssue::SignerPublicRefMismatch));

    let mut malformed_signature = signature.clone();
    malformed_signature.signature = b"not-an-ed25519-signature".to_vec();
    let malformed = adapter
        .verify(&key.public_key, VerificationInput {
            signature: &malformed_signature,
            ..verify_input
        })
        .expect_err("malformed signature outcome denied");
    assert!(malformed.to_string().contains("canonical Preserves identity"));

    let mut inconsistent_domain = signed_domain.clone();
    inconsistent_domain.bytes = b"non-canonical-domain-bytes".to_vec();
    let inconsistent = adapter
        .verify(&key.public_key, VerificationInput {
            expected_domain: &inconsistent_domain,
            ..verify_input
        })
        .expect_err("inconsistent canonical domain denied");
    assert!(inconsistent.to_string().contains("canonical Preserves identity"));

    let revoked = adapter
        .verify(&key.public_key, VerificationInput {
            signer_currentness: KeyCurrentness::Revoked,
            ..verify_input
        })
        .expect("revoked outcome");
    assert_eq!(revoked.decision.kind, VerificationDecisionKind::Deny);
    assert!(revoked.decision.issues.contains(&CryptoIdentityIssue::HandleNotCurrent(KeyCurrentness::Revoked)));
    assert!(admit_federation_verification(&revoked).is_err());

    assert_cross_purpose_and_tampered_payload_denied(&adapter, &key, verify_input);
}

/// Signing under another purpose is denied, and a signature over a different payload fails
/// verification.
fn assert_cross_purpose_and_tampered_payload_denied(
    adapter: &IrohEd25519FileAdapter<'_>,
    key: &ResolvedProductionKey,
    verify_input: VerificationInput<'_>,
) {
    let wrong_domain =
        domain(adapter.profile(), KeyPurpose::Delegation, &key.handle.handle.public_key_ref, "inventory");
    let wrong_purpose = adapter
        .sign(&key.handle.handle, &wrong_domain, &test_ref("sign-policy"))
        .expect_err("cross-purpose signing denied");
    assert!(wrong_purpose.to_string().contains("PurposeMismatch"));

    let tampered_domain = domain(
        adapter.profile(),
        key.handle.handle.purpose,
        &key.handle.handle.public_key_ref,
        "tampered-inventory",
    );
    let tampered = adapter
        .verify(&key.public_key, VerificationInput {
            expected_domain: &tampered_domain,
            ..verify_input
        })
        .expect("tamper decision");
    assert_eq!(tampered.decision.kind, VerificationDecisionKind::Deny);
    assert!(tampered.decision.issues.contains(&CryptoIdentityIssue::PayloadRefMismatch));
    assert!(tampered.decision.issues.contains(&CryptoIdentityIssue::CryptographicVerificationFailed));
}

/// The public key of a federation-origin key generated in a separate secrets namespace.
fn other_federation_public_key() -> String {
    let other_workspace = temp_dir("crypto-production-wrong-key");
    let other_namespace = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Secrets,
        &other_workspace,
    )
    .expect("other secrets namespace");
    let other_adapter = IrohEd25519FileAdapter::new(&other_namespace, profile(), test_ref("capability-file-backend"))
        .expect("other file adapter");
    let other_key = other_adapter
        .resolve_or_generate(KeyPurpose::FederationOrigin, &test_ref("generation-policy"), true)
        .expect("other federation key");
    other_key.public_key
}

// r[verify molten.crypto_identity.rotation_revocation]
// r[verify molten.crypto_identity.adapter_conformance]
#[test]
fn rotation_fences_stale_handle_and_restart_resolves_new_generation() {
    let workspace = temp_dir("crypto-production-rotation");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Secrets, &workspace)
            .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let first = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("first key");
    let request = KeyRotationRequest {
        operation_id: "rotate-evidence-key".to_string(),
        profile_ref: adapter.profile().profile.profile_ref.clone(),
        purpose: KeyPurpose::EvidenceSigning,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: first.handle.handle.backend_ref.clone(),
        old_handle_ref: first.handle.handle.handle_ref.clone(),
        old_public_key_ref: first.handle.handle.public_key_ref.clone(),
        old_generation: GENERATION_ONE,
        new_generation: GENERATION_TWO,
        policy_ref: test_ref("rotation-policy"),
        activation_boundary_ref: test_ref("activation-boundary"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(test_ref("revocation-evidence")),
    };
    let rotated = adapter.rotate(&request).expect("rotation");
    assert_eq!(rotated.handle.handle.generation, GENERATION_TWO);
    assert_ne!(
        first.public_key,
        adapter
            .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), false)
            .expect("restart")
            .public_key
    );

    let stale_domain =
        domain(adapter.profile(), KeyPurpose::EvidenceSigning, &first.handle.handle.public_key_ref, "receipt");
    let stale = adapter
        .sign(&first.handle.handle, &stale_domain, &test_ref("sign-policy"))
        .expect_err("stale handle denied");
    let stale_message = stale.to_string();
    let is_stale_handle = stale_message.contains("HandleGenerationStale") || stale_message.contains("HandleRefStale");
    assert!(is_stale_handle, "unexpected stale handle denial: {stale_message}");
}

// r[verify molten.crypto_identity.canonical_signature_binding]
// r[verify molten.crypto_identity.adapter_conformance]
#[test]
fn evidence_signature_wrapper_consumes_only_canonical_outcomes() {
    let workspace = temp_dir("crypto-evidence-signature");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Secrets, &workspace)
            .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("evidence key");
    let signed_domain =
        domain(adapter.profile(), KeyPurpose::EvidenceSigning, &key.handle.handle.public_key_ref, "receipt");
    let signature = sign_evidence_payload(&adapter, &key.handle.handle, &signed_domain, &test_ref("sign-policy"))
        .expect("evidence signature");
    let verified = adapter
        .verify(&key.public_key, VerificationInput {
            expected_domain: &signed_domain,
            signature: &signature,
            signer_currentness: KeyCurrentness::Current,
            signer_generation: key.handle.handle.generation,
            policy_ref: &test_ref("verify-policy"),
        })
        .expect("verification outcome");
    admit_evidence_verification(&verified).expect("evidence verification admission");
    assert!(admit_federation_verification(&verified).is_err());

    let revocation_ref = test_ref("evidence-key-revocation");
    let revoked = adapter
        .revoke(&key.handle.handle, &revocation_ref, &test_ref("revocation-policy"))
        .expect("revoke evidence key");
    assert_eq!(revoked.status.currentness, Some(KeyCurrentness::Revoked));
    let revoked_status =
        adapter.redacted_status(KeyPurpose::EvidenceSigning, vec![revocation_ref]).expect("revoked status");
    assert_eq!(revoked_status.status.currentness, Some(KeyCurrentness::Revoked));
    let denied = sign_evidence_payload(&adapter, &key.handle.handle, &signed_domain, &test_ref("sign-policy"))
        .expect_err("revoked key cannot sign");
    assert!(denied.to_string().contains("key is revoked"));
}
