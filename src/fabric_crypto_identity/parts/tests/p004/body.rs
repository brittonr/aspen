
fn generation_handle(profile: &Profile, generation: u64) -> OpaqueKeyHandle {
    let public_key_ref = test_ref(&format!("status-public-{generation}"));
    let backend_ref = test_ref("capability-file-backend");
    let currentness_evidence_ref = test_ref(&format!("status-currentness-{generation}"));
    key_handle(KeyHandleInput {
        profile,
        purpose: KeyPurpose::EvidenceSigning,
        generation,
        public_key_ref: &public_key_ref,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: &backend_ref,
        currentness: KeyCurrentness::Current,
        currentness_evidence_ref: &currentness_evidence_ref,
    })
    .expect("canonical key handle")
    .handle
}

fn status_input(handle: &OpaqueKeyHandle, handle_ref: Option<String>) -> AdapterDiagnosticInput {
    AdapterDiagnosticInput {
        profile_ref: handle.profile_ref.clone(),
        purpose: handle.purpose,
        generation: Some(handle.generation),
        currentness: Some(handle.currentness),
        permission_status: AdapterPermissionStatus::Restricted,
        backend_class: handle.backend_class,
        public_key_ref: Some(handle.public_key_ref.clone()),
        handle_ref,
        receipt_refs: Vec::new(),
        backend_locator: None,
        raw_error: None,
        bearer_token: None,
        private_material_present: false,
    }
}

// r[verify molten.crypto_identity.status_handle_reference]
#[test]
fn adapter_status_binds_the_resolved_handle_ref_without_key_material() {
    let workspace = temp_dir("crypto-status-handle-ref");
    let namespace = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Identity, &workspace)
        .expect("identity namespace");
    let adapter = adapter(&namespace);
    let key = adapter
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &test_ref("generation-policy"), true)
        .expect("transport key");
    let status = adapter.redacted_status(KeyPurpose::TransportEndpoint, Vec::new()).expect("status");
    assert_eq!(status.status.handle_ref.as_deref(), Some(key.handle.handle.handle_ref.as_str()));
    assert!(status.value.collect_simple_record("fabric-crypto-status-v2", None).is_some());
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(status_text.contains(CRYPTO_STATUS_HANDLE_SCHEMA));
    assert!(status_text.contains(&key.handle.handle.handle_ref));
    let secret_record = namespace
        .read(&transport_key_path().expect("transport key path"), crate::node_state::MAX_NODE_SECRET_BYTES)
        .expect("secret record");
    assert!(!status_text.as_bytes().windows(secret_record.len()).any(|window| window == secret_record));
}

// r[verify molten.crypto_identity.status_handle_reference]
#[test]
fn rotated_generations_have_distinct_status_handle_refs() {
    let profile = profile();
    let first = generation_handle(&profile, GENERATION_ONE);
    let second = generation_handle(&profile, GENERATION_TWO);
    let first_status = status(&status_input(&first, Some(first.handle_ref.clone()))).expect("first status");
    let second_status = status(&status_input(&second, Some(second.handle_ref.clone()))).expect("second status");
    assert_eq!(first_status.status.handle_ref.as_deref(), Some(first.handle_ref.as_str()));
    assert_ne!(first_status.status.handle_ref, second_status.status.handle_ref);
    assert_ne!(first_status.status_ref, second_status.status_ref);
    assert_eq!(crate::ledger::artifact_kind(&first_status.value), "fabric-crypto-status");
}

// r[verify molten.crypto_identity.status_handle_reference]
#[test]
fn status_without_a_handle_keeps_the_v1_record_and_raw_handles_are_denied() {
    let profile = profile();
    let handle = generation_handle(&profile, GENERATION_ONE);
    let legacy = status(&status_input(&handle, None)).expect("v1 status");
    assert!(legacy.value.collect_simple_record("fabric-crypto-status-v1", None).is_some());
    let legacy_text = crate::preserves_rail::to_text(&legacy.value).expect("v1 status text");
    assert!(legacy_text.contains(CRYPTO_STATUS_SCHEMA));
    assert!(!legacy_text.contains("handle-ref"));
    assert_eq!(crate::ledger::artifact_kind(&legacy.value), "fabric-crypto-status");
    assert!(status(&status_input(&handle, Some("ed25519-secret:00".to_string()))).is_err());
}
