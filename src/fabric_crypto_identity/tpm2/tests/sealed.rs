use super::*;

// r[verify molten.crypto_identity.tpm2_sealed_transport]
// r[verify molten.crypto_identity.tpm2_backend_rotation]
#[test]
#[ignore = "requires pinned nix/tpm2-identity.nix disposable swtpm rail"]
fn load_pcr_denial_secrecy_and_rotation() {
    let mut fixture = fixture::Provisioned::new();
    let adapter = Sealed::new(&fixture.scope, &fixture.executable, fixture.profile.clone(), fixture.policy.clone())
        .expect("admitted backend");
    let (handle, public_key) = adapter.resolve().expect("real PCR unseal");
    assert_eq!(public_key, fixture.expected_secret.public().to_string());
    let secret = adapter.load_transport_secret(&handle.handle).expect("actual transport consumer load");
    assert!(secret.public() == fixture.expected_secret.public());
    let _endpoint_builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal).secret_key(secret);
    let mut wrong_handle = handle.handle.clone();
    wrong_handle.generation += 1;
    assert!(matches!(adapter.load_transport_secret(&wrong_handle), Err(Tpm2IdentityFailure::HandleMismatch)));
    let mut wrong_object = fixture.policy.clone();
    wrong_object.public_object[79] ^= 1;
    let wrong_adapter = Sealed::new(&fixture.scope, &fixture.executable, fixture.profile.clone(), wrong_object)
        .expect("well-shaped other object");
    assert!(matches!(wrong_adapter.resolve(), Err(Tpm2IdentityFailure::ObjectMismatch)));
    assert_tool_binding(&fixture);
    let file_workspace =
        crate::test_support::process_workspace("tpm2-rotation-file").expect("separate file backend workspace");
    let file_scope =
        crate::node_state::Scope::open(crate::node_state::NamespaceKind::Identity, file_workspace.as_ref())
            .expect("file backend namespace");
    let file = IrohEd25519::new(&file_scope, fixture.profile.clone(), reference("file-backend")).expect("file backend");
    let old = file
        .resolve_or_generate(KeyPurpose::TransportEndpoint, &reference("generate-policy"), true)
        .expect("file source");
    assert_backend_rotations(&fixture, &adapter, &file, &old.handle.handle, &handle.handle);
    let sealed_secret = [
        ("sealed transport record", fixture.record.as_slice()),
        ("sealed seed", &fixture.record[16..]),
    ];
    let sealed_plaintext = [
        sealed_secret[0],
        sealed_secret[1],
        ("plaintext schema", b"MCKEY001".as_slice()),
    ];
    // No file is excluded from either inventory. CapabilityFile intentionally
    // persists its own plaintext schema, but must never persist the TPM's secret.
    assert_no_plaintext(&fixture.scope, &sealed_plaintext);
    assert_no_plaintext(&file_scope, &sealed_secret);
    fixture_tool(
        &fixture.executable,
        &fixture.workspace,
        &[
            "pcrextend",
            "16:sha256=1111111111111111111111111111111111111111111111111111111111111111",
            "-Q",
        ],
        bounded_exec::Input::Null,
    );
    assert!(matches!(adapter.load_transport_secret(&handle.handle), Err(Tpm2IdentityFailure::UnsealDenied)));
    assert!(file.resolve_or_generate(KeyPurpose::TransportEndpoint, &reference("read-policy"), false).is_ok());
    assert_no_plaintext(&fixture.scope, &sealed_plaintext);
    assert_no_plaintext(&file_scope, &sealed_secret);
    fixture.record.fill(0);
}

fn assert_tool_binding(fixture: &fixture::Provisioned) {
    let mut wrong_tool = fixture.policy.clone();
    wrong_tool.executable_ref = reference("unavailable tool");
    assert!(matches!(
        Sealed::new(&fixture.scope, &fixture.executable, fixture.profile.clone(), wrong_tool),
        Err(Tpm2IdentityFailure::ToolIdentityMismatch)
    ));
}

fn assert_backend_rotations(
    fixture: &fixture::Provisioned,
    adapter: &Sealed<'_>,
    file: &IrohEd25519<'_>,
    old: &OpaqueKeyHandle,
    handle: &OpaqueKeyHandle,
) {
    let forward = rotation_request(old, handle);
    let plan = plan_key_rotation(&fixture.profile.profile, old, &forward).expect("existing forward rotation plan");
    let completed = adapter.complete_rotation(&plan).expect("actual TPM destination completion");
    assert_eq!(completed.outcome.new_generation, 2);
    assert_eq!(completed.outcome.old_key_currentness, KeyCurrentness::Revoked);
    let mut wrong_destination = handle.clone();
    wrong_destination.backend_ref = reference("wrong destination PCR policy");
    assert!(complete_key_rotation(&plan, &wrong_destination).is_err());
    let mut next_file = old.clone();
    next_file.generation = 3;
    let prepared = file.rotate(&rotation_request(old, &next_file)).expect("actual file destination key");
    let reverse = rotation_request(handle, &prepared.handle.handle);
    let reverse_plan =
        plan_key_rotation(&fixture.profile.profile, handle, &reverse).expect("existing reverse rotation plan");
    let reversed = complete_key_rotation(&reverse_plan, &prepared.handle.handle).expect("actual file completion");
    assert_eq!(reversed.new_generation, 3);
}
