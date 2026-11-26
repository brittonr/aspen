// Revocation scope across key backends and accumulation of revocation records.

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
