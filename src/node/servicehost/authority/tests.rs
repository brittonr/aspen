struct Fixture {
    temporary: tempfile::TempDir,
    root: crate::node_state::DirectoryRoot,
    bundle: String,
    trust: super::wire::Trust,
    credential: super::wire::Credential,
    request: crate::node_service_hosting::OperationRequest,
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().expect("operator authority directory");
    let root = crate::node_state::DirectoryRoot::open_existing(temporary.path()).expect("operator root");
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
    let audience = basalt::ucan::AudienceDid::from(basalt::ucan::TokenSigner::issuer(&holder).expect("holder DID"));
    let bundle = crate::preserves_rail::content_ref_from_bytes(b"admitted closure");
    let request = crate::node_service_hosting::OperationRequest {
        operation: "install".into(),
        subject: "group-a".into(),
        logical_tick: 5,
        state_ref: crate::preserves_rail::content_ref_from_bytes(b"state"),
        input_ref: crate::preserves_rail::content_ref_from_bytes(b"actual install payload"),
    };
    let capabilities = basalt::ucan::CapabilitySet::new(vec![
        basalt::ucan::CapabilityDocument::new(super::resource(&bundle), request.operation.clone()).expect("capability"),
    ])
    .expect("capability set");
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(100, 200).expect("time bounds"),
    )
    .expect("signed token");
    let credential = super::sign_credential(
        super::SigningInput {
            bundle_root: &bundle,
            request: &request,
            token: token.as_str(),
            audience: &audience,
            nonce: [17; 32],
        },
        &holder,
    )
    .expect("holder signature");
    let trust = super::wire::Trust {
        issuer: basalt::ucan::TokenSigner::issuer(&issuer).expect("issuer DID").as_str().into(),
        issuer_key: basalt::ucan::derive_ed25519_public_key(&[71; 32]).expect("issuer key"),
        holder: audience.as_str().into(),
        holder_key: basalt::ucan::derive_ed25519_public_key(&[73; 32]).expect("holder key"),
        policy: r#"{"backends":{},"contracts":{"node-service-host":{"abilities":["install"],"description":"operator policy","id":"node-service-host","resource_prefixes":["molten://node-service-host/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#.into(),
        not_before: 100, expires: 200, revoked: Vec::new(),
    };
    Fixture {
        temporary,
        root,
        bundle,
        trust,
        credential,
        request,
    }
}

fn verify(
    f: &Fixture,
    request: &crate::node_service_hosting::OperationRequest,
    bundle: &str,
) -> crate::error::Result<String> {
    super::boundaries::verify(&f.trust, &f.credential, super::boundaries::Verification {
        root: &f.root,
        now: 150,
        resource: &super::resource(bundle),
        ability: &request.operation,
        contract: "node-service-host",
        operation: super::operation_binding(bundle, request)?,
    })
}

#[test]
fn signed_invocation_rejects_every_binding_substitution_and_persists_replay() {
    let mut f = fixture();
    super::boundaries::validate_keys(&f.trust).expect("operator keys bound to DIDs");
    let mut variants = vec![f.request.clone(); 5];
    variants[0].operation = "supervise".into();
    variants[1].subject = "group-b".into();
    variants[2].logical_tick += 1;
    variants[3].state_ref = crate::preserves_rail::content_ref_from_bytes(b"other state");
    variants[4].input_ref = crate::preserves_rail::content_ref_from_bytes(b"other payload");
    for request in variants {
        assert!(verify(&f, &request, &f.bundle).is_err());
    }
    assert!(verify(&f, &f.request, &crate::preserves_rail::content_ref_from_bytes(b"other closure")).is_err());
    let receipt = verify(&f, &f.request, &f.bundle).expect("genuine signed authority");
    crate::preserves_rail::ContentRef::parse(&receipt).expect("canonical receipt reference");
    f.root = crate::node_state::DirectoryRoot::open_existing(f.temporary.path()).expect("restart capability root");
    assert!(verify(&f, &f.request, &f.bundle).is_err(), "replay must survive reopening the authority root");
}

#[test]
fn current_revocation_and_basalt_denial_do_not_consume_valid_invocation() {
    let mut f = fixture();
    let token = basalt::ucan::CompactToken::new(&f.credential.token).expect("token");
    f.trust
        .revoked
        .push(basalt::ucan::proof_reference(&token).as_bytes().try_into().expect("proof digest"));
    assert!(verify(&f, &f.request, &f.bundle).is_err());
    f.trust.revoked.clear();
    let policy = f.trust.policy.clone();
    f.trust.policy = f.trust.policy.replace("\"install\"", "\"supervise\"");
    assert!(verify(&f, &f.request, &f.bundle).is_err());
    f.trust.policy = policy;
    f.credential.signature[0] ^= 1;
    assert!(verify(&f, &f.request, &f.bundle).is_err());
    f.credential.signature[0] ^= 1;
    verify(&f, &f.request, &f.bundle).expect("denials never reserve replay state");
}
