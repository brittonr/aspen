struct Fixture {
    temporary: tempfile::TempDir,
    root: crate::node_state::DirectoryRoot,
    selection: super::NodeObservationSelection,
    trust: crate::node_service_host::authority::wire::Trust,
    credential: crate::node_service_host::authority::wire::Credential,
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().expect("authority root");
    let root = crate::node_state::DirectoryRoot::open_existing(temporary.path()).expect("root");
    let reference = crate::preserves_rail::content_ref_from_bytes(b"selected artifact");
    let selection = super::NodeObservationSelection {
        source_validation_ref: reference.clone(),
        checked_profile_path: "checked/profile.json".into(),
        checked_profile_ref: reference.clone(),
        endpoint_directory: "provider/window".into(),
        endpoint_leaf: "snapshot".into(),
        endpoint_content_ref: reference.clone(),
        epoch: 7,
        authorization_tick: 3,
        boot_id: "12345678-1234-1234-1234-123456789abc".into(),
        onix_receipt_ref: reference,
    };
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([81; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([83; 32]);
    let audience = basalt::ucan::AudienceDid::from(basalt::ucan::TokenSigner::issuer(&holder).expect("holder"));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_secs();
    let capabilities = basalt::ucan::CapabilitySet::new(vec![
        basalt::ucan::CapabilityDocument::new(
            super::node_observation_resource(&selection).expect("resource"),
            "read".to_owned(),
        )
        .expect("grant"),
    ])
    .expect("set");
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(now - 60, now + 3600).expect("time"),
    )
    .expect("token");
    let credential = super::sign_node_observation_credential(
        super::ObserverSigningInput {
            selection: &selection,
            token: token.as_str(),
            audience: &audience,
            nonce: [29; 32],
        },
        &holder,
    )
    .expect("signed holder proof");
    let trust = crate::node_service_host::authority::wire::Trust {
        issuer: basalt::ucan::TokenSigner::issuer(&issuer).expect("issuer").as_str().into(),
        issuer_key: basalt::ucan::derive_ed25519_public_key(&[81; 32]).expect("key"),
        holder: audience.as_str().into(),
        holder_key: basalt::ucan::derive_ed25519_public_key(&[83; 32]).expect("key"),
        policy: r#"{"backends":{},"contracts":{"linux-observation-reader":{"abilities":["read"],"description":"observer","id":"linux-observation-reader","resource_prefixes":["molten://linux-observation/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#.into(),
        not_before: now - 60, expires: now + 3600, revoked: Vec::new(),
    };
    let f = Fixture {
        temporary,
        root,
        selection,
        trust,
        credential,
    };
    provision(&f);
    f
}

fn provision(f: &Fixture) {
    std::fs::write(
        f.temporary.path().join("linux-observation-trust"),
        super::encode_node_observer_trust(&f.trust).expect("trust envelope"),
    )
    .expect("trust write");
    let reference = super::node_observation_selection_ref(&f.selection).expect("selection");
    std::fs::write(
        f.temporary.path().join(format!("linux-observation-credential-{}.preserves", &reference[7..])),
        super::encode_node_observer_credential(&f.credential).expect("credential envelope"),
    )
    .expect("credential write");
}

fn verify(f: &Fixture, selection: &super::NodeObservationSelection) -> crate::error::Result<String> {
    crate::node_service_host::authority::boundaries::verify(
        &f.trust,
        &f.credential,
        crate::node_service_host::authority::boundaries::Verification {
            root: &f.root,
            now: f.trust.not_before + 60,
            resource: &super::node_observation_resource(selection)?,
            ability: "read",
            contract: "linux-observation-reader",
            operation: super::node_observation_binding(selection)?,
        },
    )
}

#[test]
fn exact_selection_signature_rejects_substitution_and_durable_replay() {
    let mut f = fixture();
    let mut variants = vec![f.selection.clone(); 10];
    let other = crate::preserves_rail::content_ref_from_bytes(b"substitution");
    variants[0].source_validation_ref = other.clone();
    variants[1].checked_profile_path = "other/profile.json".into();
    variants[2].checked_profile_ref = other.clone();
    variants[3].endpoint_directory = "other/provider".into();
    variants[4].endpoint_leaf = "other".into();
    variants[5].endpoint_content_ref = other.clone();
    variants[6].epoch += 1;
    variants[7].authorization_tick += 1;
    variants[8].boot_id = "22345678-1234-1234-1234-123456789abc".into();
    variants[9].onix_receipt_ref = other;
    for selection in variants {
        assert!(verify(&f, &selection).is_err());
    }
    verify(&f, &f.selection).expect("genuine invocation");
    f.root = crate::node_state::DirectoryRoot::open_existing(f.temporary.path()).expect("restart root");
    assert!(verify(&f, &f.selection).is_err());
}

#[test]
fn current_lease_rejects_revocation_and_never_reconsumes_holder_nonce() {
    let mut f = fixture();
    let now = f.trust.not_before + 60;
    let lease = super::observerlease::admit(&f.root, &f.selection, now).expect("admission");
    lease.validate_current(now).expect("first current check");
    lease.validate_current(now).expect("second check is not replay admission");
    let token = basalt::ucan::CompactToken::new(&f.credential.token).expect("token");
    f.trust.revoked.push(basalt::ucan::proof_reference(&token).as_bytes().try_into().expect("digest"));
    provision(&f);
    assert!(lease.validate_current(now).is_err());
    assert!(super::observerlease::admit(&f.root, &f.selection, now).is_err());
}

#[test]
fn bad_holder_and_native_policy_cannot_reserve_observer_nonce() {
    let mut f = fixture();
    f.credential.signature[0] ^= 1;
    assert!(verify(&f, &f.selection).is_err());
    f.credential.signature[0] ^= 1;
    let policy = f.trust.policy.clone();
    f.trust.policy = policy.replace("linux-observation-reader", "node-service-host");
    assert!(verify(&f, &f.selection).is_err());
    f.trust.policy = policy;
    verify(&f, &f.selection).expect("denials did not reserve nonce");
}

#[test]
fn selection_rejects_escape_unknown_fields_and_native_wire_envelopes() {
    let f = fixture();
    let mut value = serde_json::to_value(&f.selection).expect("selection JSON");
    value["checked_profile_path"] = "../profile".into();
    assert!(super::decode_node_observation_selection(&serde_json::to_vec(&value).expect("JSON")).is_err());
    value = serde_json::to_value(&f.selection).expect("selection JSON");
    value["permission"] = true.into();
    assert!(super::decode_node_observation_selection(&serde_json::to_vec(&value).expect("JSON")).is_err());
    assert!(
        super::decode_node_observer_trust(
            &crate::node_service_host::authority::wire::encode_trust(&f.trust).expect("native trust")
        )
        .is_err()
    );
    assert!(
        super::decode_node_observer_credential(
            &crate::node_service_host::authority::wire::encode_credential(&f.credential).expect("native credential")
        )
        .is_err()
    );
}
