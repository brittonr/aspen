fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
fn adapter(scope: &crate::node_state::Scope) -> crate::fabric_crypto_identity::IrohEd25519<'_> {
    let profile = crate::fabric_crypto_identity::profile(&crate::fabric_crypto_identity::production_ed25519_profile(
        reference("federation-test-crypto"),
        reference("federation-test-entropy"),
    ))
    .unwrap();
    crate::fabric_crypto_identity::IrohEd25519::new(scope, profile, reference("federation-test-backend")).unwrap()
}

/// Purpose, generation, revocation, freshness, and context drift must each deny an otherwise valid
/// record.
fn assert_verification_drift_denied(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    record: &super::signatures::SignedRecord,
    input: &super::signatures::VerificationInput<'_>,
) {
    assert!(
        super::signatures::verify_record(adapter, record, &super::signatures::VerificationInput {
            purpose: molten_core::federation::RecordPurpose::Response,
            ..*input
        })
        .is_err()
    );
    assert!(
        super::signatures::verify_record(adapter, record, &super::signatures::VerificationInput {
            generation: input.generation + 1,
            ..*input
        })
        .is_err()
    );
    assert!(
        super::signatures::verify_record(adapter, record, &super::signatures::VerificationInput {
            currentness: crate::fabric_crypto_identity::KeyCurrentness::Revoked,
            ..*input
        })
        .is_err()
    );
    assert!(
        super::signatures::verify_record(adapter, record, &super::signatures::VerificationInput { now: 31, ..*input })
            .is_err()
    );
    let wrong_context = reference("unrelated-verifier-context");
    assert!(
        super::signatures::verify_record(adapter, record, &super::signatures::VerificationInput {
            context_ref: &wrong_context,
            ..*input
        })
        .is_err()
    );
}

// r[verify molten.federated_pull_runtime.crypto]
#[test]
fn signed_records_deny_wrong_key_purpose_context_generation_revocation_and_corruption() {
    let workspace = crate::test_support::ProcessWorkspace::new("federation-signatures").unwrap();
    let scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &workspace).unwrap();
    let adapter = adapter(&scope);
    let policy = reference("federation-local-signature-policy");
    let context = reference("federation-local-context");
    let key = adapter
        .resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)
        .unwrap();
    let payload = super::records::delegate_payload(&super::records::DelegatePayload {
        subject_ref: reference("artifact"),
        delegate_ref: reference("delegate"),
        capability: "federation:serve".to_string(),
        policy_ref: policy.clone(),
    })
    .unwrap();
    let record = super::signatures::sign_record(&adapter, &super::signatures::SigningInput {
        profile: adapter.profile(),
        handle: &key.handle.handle,
        context_ref: &context,
        policy_ref: &policy,
        purpose: molten_core::federation::RecordPurpose::Delegate,
        issued_at: 10,
        expires_at: 30,
        payload: &payload,
    })
    .unwrap();
    let input = super::signatures::VerificationInput {
        profile: adapter.profile(),
        public_key: &key.public_key,
        signer_ref: &key.handle.handle.public_key_ref,
        generation: key.handle.handle.generation,
        currentness: crate::fabric_crypto_identity::KeyCurrentness::Current,
        context_ref: &context,
        policy_ref: &policy,
        purpose: molten_core::federation::RecordPurpose::Delegate,
        now: 20,
        max_age: 20,
    };
    let bytes = super::wire::encode(&record, 4096).unwrap();
    let decoded = super::wire::decode(&bytes, &input, 4096).unwrap();
    let accepted = super::signatures::verify_record(&adapter, &decoded, &input).unwrap();
    assert_eq!(accepted.decision.kind, crate::fabric_crypto_identity::VerificationDecisionKind::Accept);
    assert_verification_drift_denied(&adapter, &record, &input);
    let other = crate::test_support::ProcessWorkspace::new("federation-wrong-key").unwrap();
    let other_scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &other).unwrap();
    let other_adapter = self::adapter(&other_scope);
    let other_key = other_adapter
        .resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)
        .unwrap();
    assert!(
        super::signatures::verify_record(&adapter, &record, &super::signatures::VerificationInput {
            public_key: &other_key.public_key,
            ..input
        })
        .is_err()
    );
    let mut corrupted = record;
    corrupted.signature.signature[0] ^= 1;
    assert!(super::signatures::verify_record(&adapter, &corrupted, &input).is_err());
    assert!(super::wire::decode(&bytes, &input, u64::try_from(bytes.len()).unwrap() - 1).is_err());
    assert!(super::wire::decode(b"malformed", &input, 4096).is_err());
}

// r[verify molten.federated_pull_runtime.crypto]
#[test]
fn production_signing_rejects_fixture_profile_even_with_a_real_key_handle() {
    let workspace = crate::test_support::ProcessWorkspace::new("federation-fixture-denial").unwrap();
    let scope = crate::node_state::Scope::open(crate::node_state::NamespaceKind::Secrets, &workspace).unwrap();
    let adapter = adapter(&scope);
    let policy = reference("federation-signing-policy");
    let context = reference("federation-context");
    let key = adapter
        .resolve_or_generate(crate::fabric_crypto_identity::KeyPurpose::FederationOrigin, &policy, true)
        .unwrap();
    let fixture = crate::fabric_crypto_identity::profile(&crate::fabric_crypto_identity::fixture_blake3_profile(
        reference("fixture"),
    ))
    .unwrap();
    let payload = crate::preserves_rail::record("candidate", Vec::new());
    assert!(
        super::signatures::sign_record(&adapter, &super::signatures::SigningInput {
            profile: &fixture,
            handle: &key.handle.handle,
            context_ref: &context,
            policy_ref: &policy,
            purpose: molten_core::federation::RecordPurpose::Announcement,
            issued_at: 10,
            expires_at: 30,
            payload: &payload,
        })
        .is_err()
    );
}
