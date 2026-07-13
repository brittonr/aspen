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
    let verified = adapter
        .verify(
            &key.public_key,
            &signed_domain,
            &signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect("verification outcome");
    assert_eq!(verified.decision.kind, VerificationDecisionKind::Accept);
    admit_federation_verification(&verified).expect("federation verification admission");

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
    let wrong_key = adapter
        .verify(
            &other_key.public_key,
            &signed_domain,
            &signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect("wrong key outcome");
    assert_eq!(wrong_key.decision.kind, VerificationDecisionKind::Deny);
    assert!(wrong_key.decision.issues.contains(&CryptoIdentityIssue::SignerPublicRefMismatch));

    let mut malformed_signature = signature.clone();
    malformed_signature.signature = b"not-an-ed25519-signature".to_vec();
    let malformed = adapter
        .verify(
            &key.public_key,
            &signed_domain,
            &malformed_signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect_err("malformed signature outcome denied");
    assert!(malformed.to_string().contains("canonical Preserves identity"));

    let mut inconsistent_domain = signed_domain.clone();
    inconsistent_domain.bytes = b"non-canonical-domain-bytes".to_vec();
    let inconsistent = adapter
        .verify(
            &key.public_key,
            &inconsistent_domain,
            &signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect_err("inconsistent canonical domain denied");
    assert!(inconsistent.to_string().contains("canonical Preserves identity"));

    let revoked = adapter
        .verify(
            &key.public_key,
            &signed_domain,
            &signature,
            KeyCurrentness::Revoked,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect("revoked outcome");
    assert_eq!(revoked.decision.kind, VerificationDecisionKind::Deny);
    assert!(revoked.decision.issues.contains(&CryptoIdentityIssue::HandleNotCurrent(KeyCurrentness::Revoked)));
    assert!(admit_federation_verification(&revoked).is_err());

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
        .verify(
            &key.public_key,
            &tampered_domain,
            &signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
        .expect("tamper decision");
    assert_eq!(tampered.decision.kind, VerificationDecisionKind::Deny);
    assert!(tampered.decision.issues.contains(&CryptoIdentityIssue::PayloadRefMismatch));
    assert!(tampered.decision.issues.contains(&CryptoIdentityIssue::CryptographicVerificationFailed));
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
    assert!(stale.to_string().contains("HandleGenerationStale") || stale.to_string().contains("HandleRefStale"));
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
        .verify(
            &key.public_key,
            &signed_domain,
            &signature,
            KeyCurrentness::Current,
            key.handle.handle.generation,
            &test_ref("verify-policy"),
        )
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

// r[verify molten.crypto_identity.fixture_profile_boundary]
// r[verify molten.crypto_identity.adapter_conformance]
#[test]
fn fixture_profile_and_missing_or_unsafe_keys_fail_closed() {
    let fixture = canonical_crypto_profile(&fixture_blake3_profile(test_ref("fixture-profile")))
        .expect("fixture profile is valid for tests");
    let workspace = temp_dir("crypto-fixture-denial");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Identity, &workspace)
            .expect("identity namespace");
    let denied = IrohEd25519FileAdapter::new(&namespace, fixture, test_ref("fixture-backend"))
        .err()
        .expect("fixture profile denied by production adapter");
    assert!(denied.to_string().contains("FixtureProfileDeniedInProduction"));
    let production_profile = profile();
    let fixture_signature = admit_signature_algorithm(&production_profile, LEGACY_FIXTURE_SIGNATURE_ALGORITHM)
        .expect_err("legacy fixture signature denied in production");
    assert!(fixture_signature.to_string().contains("denied by production"));

    let adapter = adapter(&namespace);
    let unavailable = adapter
        .resolve_or_generate(KeyPurpose::Authority, &test_ref("generation-policy"), false)
        .expect_err("missing required key denied");
    assert!(unavailable.to_string().contains("replacement generation is disabled"));

    let malformed_workspace = temp_dir("crypto-malformed-record");
    let malformed_namespace = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Secrets,
        &malformed_workspace,
    )
    .expect("malformed secrets namespace");
    let malformed_path = crate::node_state::NodeStatePath::parse("crypto-authority.key").expect("authority key path");
    malformed_namespace
        .write_restricted(&malformed_path, b"malformed", OWNER_ONLY_SECRET_FILE_MODE)
        .expect("malformed key fixture");
    let malformed_adapter =
        IrohEd25519FileAdapter::new(&malformed_namespace, profile(), test_ref("capability-file-backend"))
            .expect("malformed file adapter");
    let malformed = malformed_adapter
        .resolve_or_generate(KeyPurpose::Authority, &test_ref("generation-policy"), false)
        .expect_err("malformed key denied");
    assert!(malformed.to_string().contains("production key record has an invalid length"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        const UNSAFE_SECRET_MODE: u32 = 0o644;

        adapter
            .resolve_or_generate(KeyPurpose::Authority, &test_ref("generation-policy"), true)
            .expect("generate authority key");
        let path = workspace.join("crypto-authority.key");
        let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(UNSAFE_SECRET_MODE);
        std::fs::set_permissions(path, permissions).expect("unsafe mode");
        let unsafe_key = adapter
            .resolve_or_generate(KeyPurpose::Authority, &test_ref("generation-policy"), false)
            .expect_err("unsafe permissions denied");
        assert!(unsafe_key.to_string().contains("not owner-only"));
    }
}

// r[verify molten.crypto_identity.adapter_contract]
#[test]
fn transport_secret_is_only_resolved_for_current_transport_handle() {
    let workspace = temp_dir("crypto-transport-handle");
    let namespace =
        crate::node_state::NodeStateNamespace::open(crate::node_state::NodeStateNamespaceKind::Identity, &workspace)
            .expect("identity namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("transport key");
    let secret = adapter.load_transport_secret(&key.handle.handle).expect("current transport secret");
    assert_eq!(secret.public().to_string(), key.public_key);

    let mut wrong_purpose = key.handle.handle.clone();
    wrong_purpose.purpose = KeyPurpose::FederationOrigin;
    let denied = adapter.load_transport_secret(&wrong_purpose).expect_err("wrong purpose denied");
    assert!(denied.to_string().contains("transport-purpose"));

    adapter
        .revoke(&key.handle.handle, &test_ref("transport-revocation"), &test_ref("revocation-policy"))
        .expect("revoke transport key");
    let revoked = adapter
        .load_transport_secret(&key.handle.handle)
        .expect_err("revoked transport endpoint key denied");
    assert!(revoked.to_string().contains("key is revoked"));
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
    }
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
