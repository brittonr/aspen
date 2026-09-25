
// r[verify molten.content_store_adapter.identity_boundary]
// r[verify molten.content_store_adapter.live_sim_conformance]
#[tokio::test]
async fn live_iroh_blobs_stream_preserves_molten_identity_and_uses_opaque_admitted_transport_key() {
    let (workspace, root, manifest) = fixture_store("content-adapter-live-iroh");
    let identity_workspace = temp_dir("content-adapter-live-identity");
    let (namespace, backend_ref, material) = live_transport_identity(&identity_workspace);
    let endpoint_id = material.endpoint_id.clone();
    let profile = profile(ContentAdapterClass::IrohBlobs);
    assert!(
        publish_live_iroh_chunks(&profile, &root, &manifest.manifest_ref, LiveIrohIdentity {
            namespace: &namespace,
            endpoint_id: &endpoint_id,
            handle_ref: &test_ref("stale-handle"),
            backend_ref: &backend_ref,
        },)
        .await
        .is_err()
    );
    let mut publication = publish_live_iroh_chunks(&profile, &root, &manifest.manifest_ref, LiveIrohIdentity {
        namespace: &namespace,
        endpoint_id: &endpoint_id,
        handle_ref: &material.handle_ref,
        backend_ref: &backend_ref,
    })
    .await
    .expect("live publication");
    assert_eq!(publication.manifest().manifest_ref, manifest.manifest_ref);
    let get = command(&profile, publication.manifest(), ContentOperation::Get, None);
    let execution = execute_live_iroh_stream_get(StreamGetInput {
        profile: &profile,
        publication: &publication,
        command: &get,
        generation: GENERATION_ONE,
        retained: None,
        timeout: std::time::Duration::from_secs(LIVE_TIMEOUT_SECONDS),
    })
    .await
    .expect("live Iroh stream");
    assert_eq!(execution.state.artifact.terminal, ContentTerminal::Verified);
    assert_eq!(
        assemble_verified_content(publication.manifest(), &execution.state.artifact, &execution.verified_chunks)
            .unwrap(),
        b"aaaabbbbcccc"
    );
    assert!(!execution.backend_hint_ref.contains(&endpoint_id));

    publication.invalidate_first_locator();
    let stale = execute_live_iroh_stream_get(StreamGetInput {
        profile: &profile,
        publication: &publication,
        command: &get,
        generation: GENERATION_ONE,
        retained: None,
        timeout: std::time::Duration::from_secs(LIVE_TIMEOUT_SECONDS),
    })
    .await
    .expect("stale ticket outcome");
    assert_eq!(stale.state.artifact.terminal, ContentTerminal::Failed);
    assert_eq!(stale.state.artifact.failure, Some(ContentFailure::StaleTicket));
    assert!(stale.verified_chunks.is_empty());
    publication.shutdown().await.expect("shutdown publication");
    std::fs::remove_dir_all(workspace).expect("remove live fixture");
    std::fs::remove_dir_all(identity_workspace).expect("remove identity fixture");
}

/// A generated production Ed25519 transport identity in a fresh identity namespace, with its
/// backend ref and endpoint key material.
fn live_transport_identity(
    identity_workspace: &std::path::Path,
) -> (
    crate::node_state::NodeStateNamespace,
    String,
    crate::fabric_crypto_identity::TransportEndpointKeyMaterial,
) {
    let namespace = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Identity,
        identity_workspace,
    )
    .expect("identity namespace");
    let crypto_profile = crate::fabric_crypto_identity::canonical_crypto_profile(
        &crate::fabric_crypto_identity::production_ed25519_profile(
            test_ref("content-crypto-profile"),
            test_ref("content-crypto-entropy"),
        ),
    )
    .expect("crypto profile");
    let backend_ref = test_ref("content-identity-backend");
    let adapter =
        crate::fabric_crypto_identity::IrohEd25519FileAdapter::new(&namespace, crypto_profile, backend_ref.clone())
            .expect("crypto adapter");
    adapter
        .resolve_or_generate(
            crate::fabric_crypto_identity::KeyPurpose::TransportEndpoint,
            &test_ref("content-key-policy"),
            true,
        )
        .expect("transport identity");
    let key_path = crate::fabric_crypto_identity::transport_key_path().expect("transport key path");
    let key_record = namespace.read(&key_path, crate::node_state::MAX_NODE_SECRET_BYTES).expect("transport key record");
    let material = crate::fabric_crypto_identity::transport_endpoint_material(&key_record, &backend_ref)
        .expect("transport endpoint material");
    (namespace, backend_ref, material)
}

// r[verify molten.content_store_adapter.retention_boundary]
// r[verify molten.content_store_adapter.final_validation]
#[test]
fn status_and_backend_protection_are_redacted_bounded_and_non_authoritative() {
    let profile = profile(ContentAdapterClass::CapabilityLocal);
    let backend_label = "/private/root?ticket=secret";
    let status = bounded_content_status(StatusInput {
        profile: &profile,
        generation: GENERATION_ONE,
        active_operations: 0,
        queued_bytes: 0,
        terminal_counts: vec![(ContentTerminal::Verified, 1)],
        backend_label,
        issues: Vec::new(),
    })
    .expect("status");
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(!status_text.contains(backend_label));
    let protection =
        backend_protection_status(&profile, &test_ref("protected-manifest"), false).expect("unprotect status");
    assert_eq!(protection.artifact.terminal, ContentTerminal::Verified);
    assert_eq!(backend_protection_effect_grants_authority(), ContentAuthorityDecision::Deny);
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove stale fixture");
    }
    std::fs::create_dir_all(&dir).expect("create fixture");
    dir
}
