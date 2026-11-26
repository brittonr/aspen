
// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn verify_preserves_mismatched_signer_metadata_and_denies() {
    let workspace = temp_dir("crypto-signer-metadata-mismatch");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &workspace)
        .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::FederationOrigin, &test_ref("generation-policy"), true)
        .expect("federation key");
    let signed_domain =
        domain(adapter.profile(), KeyPurpose::FederationOrigin, &key.handle.handle.public_key_ref, "inventory");
    let signature =
        adapter.sign(&key.handle.handle, &signed_domain, &test_ref("sign-policy")).expect("federation signature");
    let verify_policy_ref = test_ref("verify-policy");
    let verify_input = VerificationInput {
        expected_domain: &signed_domain,
        signature: &signature,
        signer_currentness: KeyCurrentness::Current,
        signer_generation: key.handle.handle.generation,
        policy_ref: &verify_policy_ref,
    };
    let accepted = adapter.verify(&key.public_key, verify_input).expect("positive control");
    assert_eq!(accepted.decision.kind, VerificationDecisionKind::Accept);

    // Canonically re-sealed outcome whose metadata names another signer than the verifying key.
    let mut forged = signature.clone();
    forged.metadata.signer_public_ref = test_ref("forged-signer");
    forged.value = signature_value(&forged.metadata, &forged.signature);
    forged.outcome_ref = crate::preserves_rail::canonical_hash(&forged.value).expect("forged outcome ref");
    let denied = adapter
        .verify(&key.public_key, VerificationInput {
            signature: &forged,
            ..verify_input
        })
        .expect("forged metadata decision");
    assert_eq!(denied.decision.kind, VerificationDecisionKind::Deny);
    assert!(denied.decision.issues.contains(&CryptoIdentityIssue::SignerPublicRefMismatch));
    assert!(!denied.decision.issues.contains(&CryptoIdentityIssue::CryptographicVerificationFailed));
}

// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn signing_never_pairs_a_handle_with_a_concurrently_rotated_secret() {
    let workspace = temp_dir("crypto-sign-rotation-seam");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &workspace)
        .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("first key");
    let signed_domain =
        domain(adapter.profile(), KeyPurpose::EvidenceSigning, &key.handle.handle.public_key_ref, "receipt");
    // Rotate exactly between the admitted observation and the signature.
    file_adapter::seam::set("sign-after-observe", |adapter| {
        let current = adapter
            .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), false)
            .expect("current key");
        adapter
            .rotate(&rotation_request(adapter, &current.handle.handle, KeyPurpose::EvidenceSigning))
            .expect("concurrent rotation");
    });
    let signature = adapter
        .sign(&key.handle.handle, &signed_domain, &test_ref("sign-policy"))
        .expect("signature under the admitted handle");
    let rotated = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), false)
        .expect("rotated key");
    assert_ne!(rotated.public_key, key.public_key, "the seam must have rotated the key");
    let verified = adapter
        .verify(&key.public_key, VerificationInput {
            expected_domain: &signed_domain,
            signature: &signature,
            signer_currentness: KeyCurrentness::Current,
            signer_generation: key.handle.handle.generation,
            policy_ref: &test_ref("verify-policy"),
        })
        .expect("verification outcome");
    assert_eq!(
        verified.decision.kind,
        VerificationDecisionKind::Accept,
        "the signature must come from the admitted handle's key, not the rotated one: {:?}",
        verified.decision.issues
    );
}

// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn concurrent_revocations_serialize_and_keep_every_record() {
    let workspace = temp_dir("crypto-revocation-rmw");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Identity, &workspace)
        .expect("identity namespace");
    let adapter = adapter(&namespace);
    let first = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("key A");
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    // B reports either that it is at the update-lock acquisition (it cannot read before A publishes)
    // or that it is at its read without having had to wait for A (the lost-update interleaving).
    let (event_tx, event_rx) = std::sync::mpsc::channel::<&'static str>();
    let a_handle_ref = first.handle.handle.handle_ref.clone();
    std::thread::scope(|threads| {
        let adapter_ref = &adapter;
        let concurrent = threads.spawn(move || {
            go_rx.recv().expect("start concurrent revoker");
            let at_lock = event_tx.clone();
            molten_node_host::node_state::publication_seam::set_before_lock(move |name| {
                if name.ends_with("node-endpoint.secret.revoked.update-lock") {
                    let _reported = at_lock.send("at-update-lock");
                }
            });
            // At B's actual read: observe exactly the set B is about to read, report, then require that
            // A's record is already in it. Without the update lock this runs while A is paused between
            // its read and its publication, sees the pre-A set, and fails.
            file_adapter::seam::set("revoke-before-read", move |adapter| {
                let observed = file_adapter::read_key_revocations(adapter.test_namespace(), KeyPurpose::TransportEndpoint)
                    .expect("revocation set B is about to read");
                let _reported = event_tx.send("before-read");
                assert!(
                    observed.iter().any(|record| record.handle_ref == a_handle_ref),
                    "B read the revocation set before A's revocation was published"
                );
            });
            // Rotate to B while A's revocation is paused between its read and its publication, then
            // revoke B.
            let current = adapter_ref
                .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), false)
                .expect("A is still current");
            let rotated = adapter_ref
                .rotate(&rotation_request(adapter_ref, &current.handle.handle, KeyPurpose::TransportEndpoint))
                .expect("rotate to B");
            adapter_ref
                .revoke(&rotated.handle.handle, &test_ref("b-revocation"), &test_ref("revocation-policy"))
                .expect("revoke B");
            molten_node_host::node_state::publication_seam::clear_before_lock();
            // A's one-shot hook (and its receiver) may already be gone once A has released the lock.
            let _reported = done_tx.send(());
        });
        // A has read the set and is paused before publishing. B is released and A waits for B's first
        // boundary event: with the lock it is the update-lock acquisition (B then cannot read until A
        // publishes); without it, it is B's read of the pre-A set, whose hook fails the test.
        file_adapter::seam::set("revoke-after-read", move |_adapter| {
            go_tx.send(()).expect("release concurrent revoker");
            let first_event = event_rx
                .recv_timeout(std::time::Duration::from_secs(30))
                .expect("B reaches the update lock or its read");
            assert_eq!(first_event, "at-update-lock", "B reached its read while A held the revocation set");
            assert!(done_rx.try_recv().is_err(), "B's revocation must not complete while A holds the lock");
        });
        adapter
            .revoke(&first.handle.handle, &test_ref("a-revocation"), &test_ref("revocation-policy"))
            .expect("revoke A");
        concurrent.join().expect("concurrent revoker");
    });
    let records = file_adapter::read_key_revocations(&namespace, KeyPurpose::TransportEndpoint).expect("set");
    assert_eq!(records.len(), 2, "both revocations must persist: {records:?}");
    assert!(
        file_adapter::require_key_not_revoked(&namespace, file_adapter::KeyRevocationSubject::of(&first.handle.handle))
            .is_err(),
        "A must stay revoked"
    );
    let status = adapter.redacted_status(KeyPurpose::TransportEndpoint, Vec::new()).expect("B status");
    assert_eq!(status.status.currentness, Some(KeyCurrentness::Revoked));
}

// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn a_full_revocation_set_denies_further_revocation_without_breaking_resolution() {
    let workspace = temp_dir("crypto-revocation-bound");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &workspace)
        .expect("secrets namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("evidence key");
    let historical = (0..file_adapter::MAX_KEY_REVOCATIONS)
        .map(|index| file_adapter::KeyRevocation {
            handle_ref: test_ref(&format!("retired-handle-{index}")),
            public_key_ref: test_ref(&format!("retired-key-{index}")),
            generation: GENERATION_ONE,
            evidence_ref: test_ref("retired-evidence"),
            policy_ref: test_ref("retired-policy"),
        })
        .collect::<Vec<_>>();
    let marker_path = file_adapter::revocation_path(KeyPurpose::EvidenceSigning).expect("marker path");
    let marker = crate::preserves_rail::canonical_bytes(&file_adapter::key_revocations_value(&historical))
        .expect("marker bytes");
    namespace.write_restricted(&marker_path, &marker, OWNER_ONLY_SECRET_FILE_MODE).expect("full set");
    adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), false)
        .expect("a full set still resolves the current key");

    let denied = adapter
        .revoke(&key.handle.handle, &test_ref("one-too-many"), &test_ref("revocation-policy"))
        .expect_err("the 65th revocation must be denied before publication");
    assert!(denied.to_string().contains("revocation set is full"), "{denied}");
    assert_eq!(
        file_adapter::read_key_revocations(&namespace, KeyPurpose::EvidenceSigning).expect("set").len(),
        file_adapter::MAX_KEY_REVOCATIONS
    );
    adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), false)
        .expect("resolution still works after the denied revocation");
}

fn rotation_request(adapter: &IrohEd25519<'_>, current: &OpaqueKeyHandle, purpose: KeyPurpose) -> KeyRotationRequest {
    KeyRotationRequest {
        operation_id: format!("rotate-{}", purpose.as_str()),
        profile_ref: adapter.profile().profile.profile_ref.clone(),
        purpose,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: current.backend_ref.clone(),
        old_handle_ref: current.handle_ref.clone(),
        old_public_key_ref: current.public_key_ref.clone(),
        old_generation: current.generation,
        new_generation: current.generation + 1,
        policy_ref: test_ref("rotation-policy"),
        activation_boundary_ref: test_ref("activation-boundary"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(test_ref("revocation-evidence")),
    }
}

// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn file_key_revocation_does_not_deny_rotated_tpm2_handle() {
    let workspace = temp_dir("crypto-revocation-tpm2-scope");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Identity, &workspace)
        .expect("identity namespace");
    let adapter = adapter(&namespace);
    let file_key = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("file transport key");
    adapter
        .revoke(&file_key.handle.handle, &test_ref("file-key-revocation"), &test_ref("revocation-policy"))
        .expect("revoke file key");
    let file_denied = file_adapter::require_key_not_revoked(
        &namespace,
        file_adapter::KeyRevocationSubject::of(&file_key.handle.handle),
    )
    .expect_err("revoked file handle stays denied");
    assert!(file_denied.to_string().contains("key is revoked"));

    let successor = iroh::SecretKey::generate();
    let successor_public_ref = crate::preserves_rail::content_ref_from_bytes(successor.public().as_bytes());
    let sealed_backend_ref = test_ref("tpm2-sealed-backend");
    let sealed_handle = |public_key_ref: &str| {
        key_handle(KeyHandleInput {
            profile: adapter.profile(),
            purpose: KeyPurpose::TransportEndpoint,
            generation: GENERATION_TWO,
            public_key_ref,
            backend_class: KeyBackendClass::Tpm2Sealed,
            backend_ref: &sealed_backend_ref,
            currentness: KeyCurrentness::Current,
            currentness_evidence_ref: &test_ref("tpm2-currentness"),
        })
        .expect("sealed handle")
        .handle
    };
    let rotated = sealed_handle(&successor_public_ref);
    file_adapter::require_key_not_revoked(&namespace, file_adapter::KeyRevocationSubject::of(&rotated))
        .expect("rotated TPM2 key stays usable after the old file key is revoked");
    // The revoked key material stays revoked even if re-exposed under a sealed handle.
    let rewrapped = sealed_handle(&file_key.handle.handle.public_key_ref);
    assert!(
        file_adapter::require_key_not_revoked(&namespace, file_adapter::KeyRevocationSubject::of(&rewrapped))
            .is_err()
    );
}

// r[verify molten.authority.crypto_adapter_exact_verification_and_scoped_revocation]
#[test]
fn revocation_records_accumulate_and_malformed_markers_fail_closed() {
    let workspace = temp_dir("crypto-revocation-accumulate");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Identity, &workspace)
        .expect("identity namespace");
    let adapter = adapter(&namespace);
    let key_path = transport_key_path().expect("transport key path");
    let first = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("first key");
    let first_bytes = namespace.read(&key_path, crate::node_state::MAX_NODE_SECRET_BYTES).expect("first key bytes");
    adapter
        .revoke(&first.handle.handle, &test_ref("first-revocation"), &test_ref("revocation-policy"))
        .expect("revoke first key");

    // An operator-installed replacement is a distinct key and resolves.
    namespace
        .write_restricted(&key_path, &generate_transport_key_record(), OWNER_ONLY_SECRET_FILE_MODE)
        .expect("install replacement key");
    let replacement = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), false)
        .expect("replacement key is not covered by the first revocation");
    assert_ne!(replacement.handle.handle.public_key_ref, first.handle.handle.public_key_ref);
    adapter
        .revoke(&replacement.handle.handle, &test_ref("second-revocation"), &test_ref("revocation-policy"))
        .expect("revoke replacement key");

    // Revoking the second key must not erase the first revocation.
    namespace
        .write_restricted(&key_path, &first_bytes, OWNER_ONLY_SECRET_FILE_MODE)
        .expect("restore first key");
    let restored = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), false)
        .expect_err("restored first key stays revoked");
    assert!(restored.to_string().contains("key is revoked"));

    let revocation_path = crate::node_state::RelativePath::parse("node-endpoint.secret.revoked").expect("marker");
    namespace
        .write_restricted(&revocation_path, b"not-a-revocation-record", OWNER_ONLY_SECRET_FILE_MODE)
        .expect("malformed marker");
    assert!(
        adapter
            .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), false)
            .is_err()
    );
}
