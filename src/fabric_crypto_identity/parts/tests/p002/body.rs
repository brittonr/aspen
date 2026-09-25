
/// A revoked signer is denied with an explained currentness cause, and an unknown currentness ref
/// is refused.
fn assert_currentness_enforced(statement: &MoltenArtifactAuthStatementInput<'_>, signed: &SignedArtifactAuthStatement) {
    let mut revoked_request = statement.request.clone();
    revoked_request.signer_currentness = KeyCurrentness::Revoked;
    let revoked_input = MoltenArtifactAuthStatementInput {
        request: &revoked_request,
        ..*statement
    };
    let revoked = evaluate_artifact_auth_shell_dual_run(&revoked_input, signed).expect("revoked decision");
    assert_eq!(revoked.dual_run.legacy.kind, VerificationDecisionKind::Deny);
    assert!(revoked.dual_run.standalone.as_ref().is_some_and(|decision| !decision.passed));
    assert!(revoked.dual_run.compatibility.case_explained);
    assert!(revoked.dual_run.compatibility.standalone_failure_causes.contains(&"currentness".to_string()));

    let unknown_currentness = MoltenArtifactAuthStatementInput {
        currentness_ref: "unknown-currentness",
        ..*statement
    };
    let unknown = evaluate_artifact_auth_shell_dual_run(&unknown_currentness, signed)
        .expect_err("unknown currentness reference denied");
    assert!(unknown.to_string().contains("currentness_ref:expected-blake3-ref"));
}

// r[verify molten.artifact_auth_operational_receipt.identity]
// r[verify molten.artifact_auth_operational_receipt.persistence]
// r[verify molten.artifact_auth_operational_receipt.replay]
// r[verify molten.artifact_auth_operational_receipt.authority]
#[test]
fn artifact_auth_operational_receipt_survives_restart_and_rejects_rotation() {
    let workspace = temp_dir("artifact-auth-operational-restart");
    let root = crate::node_state::NodeStateRoot::open(&workspace).expect("node state root");
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let receipts = root.receipts().expect("receipts namespace");
    let initial_adapter = adapter(&secrets);
    let key = initial_adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("evidence key");
    let (request, signing_policy_ref, receipt) = operational_receipt_fixture(&initial_adapter, &key);
    let path = write_artifact_auth_operational_receipt(&receipts, &receipt).expect("write receipt");

    let reopened_root = crate::node_state::NodeStateRoot::open_existing(&workspace).expect("reopen node state");
    let reopened_secrets = reopened_root.secrets().expect("reopened secrets");
    let reopened_receipts = reopened_root.receipts().expect("reopened receipts");
    let reopened_adapter = adapter(&reopened_secrets);
    let reopened_key = reopened_adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &signing_policy_ref, false)
        .expect("reopen current key");
    let statement = MoltenArtifactAuthStatementInput {
        profile: &reopened_adapter.profile().profile,
        request: &request,
        producer_id: "molten",
        key_id: "evidence-signing-key",
        currentness_ref: &reopened_key.handle.handle.currentness_evidence_ref,
    };
    let input = MoltenArtifactAuthShellInput {
        statement,
        handle: &reopened_key.handle.handle,
        signing_policy_ref: &signing_policy_ref,
    };
    let replayed = replay_artifact_auth_operational_receipt(&reopened_adapter, &reopened_receipts, &path, &input)
        .expect("replay receipt after restart");
    assert!(replayed.dual_run.standalone.as_ref().is_some_and(|decision| decision.passed));
    assert!(replayed.dual_run.compatibility.legacy_authoritative);
    assert!(!replayed.dual_run.compatibility.standalone_authority_admitted);

    let rotation = KeyRotationRequest {
        operation_id: "rotate-after-operational-receipt".to_string(),
        profile_ref: reopened_adapter.profile().profile.profile_ref.clone(),
        purpose: KeyPurpose::EvidenceSigning,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref: reopened_key.handle.handle.backend_ref.clone(),
        old_handle_ref: reopened_key.handle.handle.handle_ref.clone(),
        old_public_key_ref: reopened_key.handle.handle.public_key_ref.clone(),
        old_generation: reopened_key.handle.handle.generation,
        new_generation: reopened_key.handle.handle.generation.saturating_add(1),
        policy_ref: test_ref("rotation-policy"),
        activation_boundary_ref: test_ref("activation-boundary"),
        overlap: RotationOverlapPolicy::None,
        revocation_evidence_ref: Some(test_ref("rotation-evidence")),
    };
    reopened_adapter.rotate(&rotation).expect("rotate key");
    let stale = replay_artifact_auth_operational_receipt(&reopened_adapter, &reopened_receipts, &path, &input)
        .expect_err("rotated key blocks old receipt replay");
    assert!(stale.to_string().contains("current key state drifted"));
}

// r[verify molten.artifact_auth_operational_receipt.persistence]
// r[verify molten.artifact_auth_operational_receipt.replay]
#[test]
fn artifact_auth_operational_receipt_rejects_namespace_revocation_and_tamper() {
    let workspace = temp_dir("artifact-auth-operational-negative");
    let root = crate::node_state::NodeStateRoot::open(&workspace).expect("node state root");
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let receipts = root.receipts().expect("receipts namespace");
    let adapter = adapter(&secrets);
    let key = adapter
        .resolve_or_generate(KeyPurpose::EvidenceSigning, &test_ref("generation-policy"), true)
        .expect("evidence key");
    let (request, signing_policy_ref, receipt) = operational_receipt_fixture(&adapter, &key);
    let wrong_namespace = write_artifact_auth_operational_receipt(&secrets, &receipt)
        .expect_err("secrets namespace cannot store public operational receipts");
    assert!(wrong_namespace.to_string().contains("require receipts namespace"));
    let path = write_artifact_auth_operational_receipt(&receipts, &receipt).expect("write receipt");
    let statement = MoltenArtifactAuthStatementInput {
        profile: &adapter.profile().profile,
        request: &request,
        producer_id: "molten",
        key_id: "evidence-signing-key",
        currentness_ref: &key.handle.handle.currentness_evidence_ref,
    };
    let input = MoltenArtifactAuthShellInput {
        statement,
        handle: &key.handle.handle,
        signing_policy_ref: &signing_policy_ref,
    };
    adapter
        .revoke(&key.handle.handle, &test_ref("revocation-evidence"), &test_ref("revocation-policy"))
        .expect("revoke key");
    let revoked = replay_artifact_auth_operational_receipt(&adapter, &receipts, &path, &input)
        .expect_err("revoked key blocks replay");
    assert!(revoked.to_string().contains("key is revoked"));

    receipts.write(&path, b"{\"schema\":\"tampered\"}").expect("replace test receipt bytes");
    let tampered = read_artifact_auth_operational_receipt(&receipts, &path).expect_err("malformed receipt is rejected");
    assert!(tampered.to_string().contains("receipt parsing failed"));
}

fn operational_receipt_fixture(
    adapter: &IrohEd25519FileAdapter<'_>,
    key: &ResolvedProductionKey,
) -> (VerificationRequest, String, MoltenArtifactAuthOperationalReceipt) {
    let signed_domain =
        domain(adapter.profile(), KeyPurpose::EvidenceSigning, &key.handle.handle.public_key_ref, "receipt");
    let legacy_signature =
        sign_evidence_payload(adapter, &key.handle.handle, &signed_domain, &test_ref("legacy-sign-policy"))
            .expect("legacy evidence signature");
    let request = VerificationRequest {
        operation_id: "verify-artifact-auth-operational-receipt".to_string(),
        profile_ref: adapter.profile().profile.profile_ref.clone(),
        expected_domain: signed_domain.domain,
        observed: legacy_signature.metadata,
        cryptographic_verification_passed: true,
        signer_currentness: KeyCurrentness::Current,
        signer_generation: key.handle.handle.generation,
        policy_ref: test_ref("verify-policy"),
    };
    let signing_policy_ref = test_ref("artifact-auth-sign-policy");
    let statement = MoltenArtifactAuthStatementInput {
        profile: &adapter.profile().profile,
        request: &request,
        producer_id: "molten",
        key_id: "evidence-signing-key",
        currentness_ref: &key.handle.handle.currentness_evidence_ref,
    };
    let input = MoltenArtifactAuthShellInput {
        statement,
        handle: &key.handle.handle,
        signing_policy_ref: &signing_policy_ref,
    };
    let signed = sign_artifact_auth_for_dual_run(adapter, &input).expect("exact standalone signature");
    let report = evaluate_artifact_auth_shell_dual_run(&statement, &signed).expect("exact standalone verification");
    let receipt = build_artifact_auth_operational_receipt(&input, &signed, &report).expect("operational receipt");
    (request, signing_policy_ref, receipt)
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
